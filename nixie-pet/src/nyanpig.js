window.__nixieSoundEnabled = false;
var bubbleHideTimer = null;
var celebrationClearTimer = null;
/** 与 `applyCelebrationTier` 配合：清除庆祝时递增，避免过期的 pause 定时器再挂上 */
var celebrationPauseTimer = null;
var celebrationMotionEpoch = 0;
/** 与 pet_core::SUCCESS_HOLD_MS 保持同步：`data-celebration-tier` 挂起时长 */
var CELEBRATION_ATTR_HOLD_MS = 4500;
/** 庆祝 layer 生效后再暂停 jagger/彩虹，避免 0% 关键帧附近与双 rAF 叠加造成「拖尾与猪先冻住」 */
var CELEBRATION_PAUSE_MS = 100;
/**
 * 庆祝动画结束后（animationend）解除 pause-motion，避免与 CELEBRATION_ATTR_HOLD_MS 同长导致整段 success 冻结。
 * prefers-reduced-motion 下不挂 pause（见 celebrationPauseTimer）。
 */
function wireCelebrationPauseRelease(pet, capturedEpoch) {
    var layer = pet.querySelector('.pet-celebrate-layer');
    if (!layer) return;
    function onEnd(ev) {
        var n = ev && ev.animationName != null ? String(ev.animationName) : '';
        if (n.indexOf('cele-') === -1) return;
        if (capturedEpoch !== celebrationMotionEpoch) return;
        layer.removeEventListener('animationend', onEnd);
        pet.removeAttribute('data-celebration-pause-motion');
    }
    layer.addEventListener('animationend', onEnd);
}
/** 框样式已透明；要再看绿/红参考时在 CSS .pet-look-debug 改回颜色，并置 true */
var SHOW_PET_LOOK_FIELD_DEBUG = false;
/** 转身后再横移的像素（越大越像在「跑过去」） */
var PET_LOOK_NUDGE_PX = 14;
/** 判定区内相对猪中心滞回，略大则不那么神经质 */
var LOOK_HYST_PX = 18;
var petFacingRaf = 0;
var lastPointer = { x: 0, y: 0 };
/** macOS/Windows：仅在外圈内由 Rust 注入坐标；离开外圈时 nativePointerOutside 置 false */
var pointerPollInOuter = false;
var petVisualEl = null;
var petLookShiftEl = null;
/** 'idle' | 'centering' | 'turning' | 'shifting' — 先 translateX→0 原地，再 scaleX，再挪到目标侧 */
var lookAnimPhase = 'idle';
var lookPendingFacing = null;
/** 仅在 idle 时与 data-facing 一致；转身/横移过程中用来和鼠标 desired 比较，避免误判「已经朝向目标」 */
var lookIdleFacing = 'right';
/** 滞回死区内沿用上一次在区内的左右判定，避免贴中线时来回改 desired */
var lookHystSide = 'right';
/** 防止 WebKit 在 data-facing 未变时不触发 transition 导致永远卡在 turning */
var lookFlipSafetyTimer = null;

function clearLookFlipSafetyTimer() {
    if (lookFlipSafetyTimer) {
        clearTimeout(lookFlipSafetyTimer);
        lookFlipSafetyTimer = null;
    }
}

function syncLookNudgeToFacing() {
    var petEl = document.getElementById('pet');
    if (!petEl) return;
    var left = petEl.getAttribute('data-facing') === 'left';
    petEl.style.setProperty('--pet-look-nudge', (left ? -PET_LOOK_NUDGE_PX : PET_LOOK_NUDGE_PX) + 'px');
}

function getLookNudgePx() {
    var petEl = document.getElementById('pet');
    if (!petEl) return 0;
    var v = getComputedStyle(petEl).getPropertyValue('--pet-look-nudge').trim();
    var m = v.match(/^(-?[0-9.]+)px$/);
    return m ? parseFloat(m[1]) : 0;
}

/** 转身动画结束（或朝向本就不必变）后：处理 pending，再进入横移 */
function finishLookFlipOrAdvance() {
    var petEl = document.getElementById('pet');
    if (!petEl) return;
    clearLookFlipSafetyTimer();
    if (lookPendingFacing) {
        var p = lookPendingFacing;
        lookPendingFacing = null;
        lookIdleFacing = p;
        if (p !== petEl.getAttribute('data-facing')) {
            petEl.setAttribute('data-facing', p);
            armLookFlipSafetyTimer();
            return;
        }
    }
    lookAnimPhase = 'shifting';
    var left = petEl.getAttribute('data-facing') === 'left';
    var px = left ? -PET_LOOK_NUDGE_PX : PET_LOOK_NUDGE_PX;
    requestAnimationFrame(function() {
        requestAnimationFrame(function() {
            petEl.style.setProperty('--pet-look-nudge', px + 'px');
        });
    });
}

function armLookFlipSafetyTimer() {
    clearLookFlipSafetyTimer();
    var pet = document.getElementById('pet');
    var ms =
        pet && pet.getAttribute('data-walk-phase') === 'following' ? 360 : 900;
    lookFlipSafetyTimer = setTimeout(function() {
        lookFlipSafetyTimer = null;
        if (lookAnimPhase === 'turning') finishLookFlipOrAdvance();
    }, ms);
}

/** 开始转身：若 DOM 朝向已是目标，浏览器不会触发 transitionend，必须直接 finish */
function tryStartLookFlip(petEl) {
    lookAnimPhase = 'turning';
    var next = lookIdleFacing;
    if (petEl.getAttribute('data-facing') === next) {
        finishLookFlipOrAdvance();
    } else {
        petEl.setAttribute('data-facing', next);
        armLookFlipSafetyTimer();
    }
}

function scheduleLookFacingTurn(target) {
    var petEl = document.getElementById('pet');
    if (!petEl) return;
    if (lookAnimPhase !== 'idle') {
        lookPendingFacing = target;
        return;
    }
    if (target === lookIdleFacing) return;
    lookIdleFacing = target;
    var walkFollow = petEl.getAttribute('data-walk-phase') === 'following';
    if (walkFollow || Math.abs(getLookNudgePx()) < 1) {
        if (walkFollow) {
            petEl.style.setProperty('--pet-look-nudge', '0px');
            if (petLookShiftEl) petLookShiftEl.classList.remove('look-shift-recenter');
        }
        tryStartLookFlip(petEl);
        return;
    }
    lookAnimPhase = 'centering';
    if (petLookShiftEl) petLookShiftEl.classList.add('look-shift-recenter');
    petEl.style.setProperty('--pet-look-nudge', '0px');
}

function onPetLookFlipTransitionEnd(e) {
    if (!petVisualEl || e.target !== petVisualEl) return;
    if (e.propertyName !== 'transform') return;
    if (lookAnimPhase !== 'turning') return;
    clearLookFlipSafetyTimer();
    finishLookFlipOrAdvance();
}

function onPetLookShiftTransitionEnd(e) {
    if (!petLookShiftEl || e.target !== petLookShiftEl) return;
    if (e.propertyName !== 'transform') return;
    var petEl = document.getElementById('pet');
    if (lookAnimPhase === 'centering') {
        if (petLookShiftEl) petLookShiftEl.classList.remove('look-shift-recenter');
        if (lookPendingFacing) {
            lookIdleFacing = lookPendingFacing;
            lookPendingFacing = null;
        }
        tryStartLookFlip(petEl);
        return;
    }
    if (lookAnimPhase !== 'shifting') return;
    lookAnimPhase = 'idle';
    if (lookPendingFacing) {
        var p = lookPendingFacing;
        lookPendingFacing = null;
        if (p !== lookIdleFacing) {
            scheduleLookFacingTurn(p);
        }
    }
}

/** 溜猪：与平常转头分流；先转身再挪窗（Rust 侧 chase_move_allowed） */
var WALK_CHASE_HYST_PX = 5;
var lastWalkChasePosted = null;

/** sleeping 时不跟鼠标转头（生活感：睡着了）；被点击唤醒后 Rust 切回 Idle 即恢复 */
function petMoodSkipsPointerLook() {
    var pet = document.getElementById('pet');
    return pet && pet.classList.contains('mood-sleeping');
}

/** Rust：光标离开窗口外圈时调用，避免 lastPointer 停在外圈外仍驱动朝向 */
function nativePointerOutside() {
    var pet = document.getElementById('pet');
    if (pet && pet.getAttribute('data-walk-phase') === 'following') return;
    pointerPollInOuter = false;
}

/** Rust：逻辑坐标（与 WebView 客户区一致） */
function nativePointerLook(lx, ly) {
    if (petMoodSkipsPointerLook()) return;
    pointerPollInOuter = true;
    var pet = document.getElementById('pet');
    if (pet && pet.getAttribute('data-walk-phase') === 'following') {
        if (petFacingRaf) {
            cancelAnimationFrame(petFacingRaf);
            petFacingRaf = 0;
        }
        walkChasePointer(lx, ly);
        return;
    }
    lastPointer.x = lx;
    lastPointer.y = ly;
    if (!petFacingRaf) petFacingRaf = requestAnimationFrame(flushPetFacingFromPointer);
}

/**
 * 溜猪专用：先 scheduleLookFacingTurn 对准光标侧，再 IPC 允许 Rust 挪窗（与 flushPetFacingFromPointer 分流）
 */
function walkChasePointer(lx, ly) {
    lastPointer.x = lx;
    lastPointer.y = ly;
    var pet = document.getElementById('pet');
    if (!pet) return;
    var r = pet.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) return;
    var cx = r.left + r.width * 0.5;
    var curFacing = pet.getAttribute('data-facing') === 'left' ? 'left' : 'right';
    var desired;
    if (lx < cx - WALK_CHASE_HYST_PX) desired = 'left';
    else if (lx > cx + WALK_CHASE_HYST_PX) desired = 'right';
    else desired = curFacing;

    if (lookAnimPhase === 'idle' && pet.getAttribute('data-facing') !== desired) {
        scheduleLookFacingTurn(desired);
    }

    var allow =
        lookAnimPhase === 'idle' &&
        pet.getAttribute('data-facing') === desired;
    if (lastWalkChasePosted !== allow) {
        lastWalkChasePosted = allow;
        try {
            window.ipc.postMessage(allow ? 'walk_chase_1' : 'walk_chase_0');
        } catch (e) {}
    }
}

function flushPetFacingFromPointer() {
    petFacingRaf = 0;
    var petEl = document.getElementById('pet');
    if (!petEl) return;
    if (petMoodSkipsPointerLook()) return;
    var usePoll = window.__nixiePointerPoll === true;
    if (usePoll && !pointerPollInOuter) return;
    var clientX = lastPointer.x;
    var clientY = lastPointer.y;
    var r = petEl.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) return;
    var cx = r.left + r.width * 0.5;
    var cy = r.top + r.height * 0.5;
    var inZone;
    if (usePoll) {
        inZone = clientX >= 0 && clientY >= 0 &&
            clientX <= window.innerWidth && clientY <= window.innerHeight;
    } else {
        var st = getComputedStyle(petEl);
        var zwM = parseFloat(st.getPropertyValue('--look-zone-w')) || 2.65;
        var zhM = parseFloat(st.getPropertyValue('--look-zone-h')) || 3.05;
        var zw = r.width * zwM;
        var zh = r.height * zhM;
        inZone = clientX >= cx - zw * 0.5 && clientX <= cx + zw * 0.5 &&
            clientY >= cy - zh * 0.5 && clientY <= cy + zh * 0.5;
    }
    if (!inZone) return;
    var desired = lookHystSide;
    if (clientX > cx + LOOK_HYST_PX) desired = 'right';
    else if (clientX < cx - LOOK_HYST_PX) desired = 'left';
    lookHystSide = desired;
    if (desired !== lookIdleFacing) scheduleLookFacingTurn(desired);
}

function queuePetFacingFromPointer(e) {
    if (petMoodSkipsPointerLook()) return;
    var pet = document.getElementById('pet');
    if (pet && pet.getAttribute('data-walk-phase') === 'following') return;
    lastPointer.x = e.clientX;
    lastPointer.y = e.clientY;
    if (!petFacingRaf) petFacingRaf = requestAnimationFrame(flushPetFacingFromPointer);
}
/** Git 分支名（如 feature/xxx）仅作角标；过长时截断，避免像「又念了一整句台词」 */
/** 定时刷新：只更新 Hook 小圆点（git 分支提示见 showGitTip，仅仓库变化时由 Rust 推送） */
function syncNativeHints(hasExt) {
    var dot = document.getElementById('ext-dot');
    if (hasExt) dot.classList.add('active');
    else dot.classList.remove('active');
}

/** 启动后短延迟再播 mood，避免首帧与本地 sleeping→idle 叠音 */
var soundGateReady = false;
var nixieAudioCtx = null;

function getNixieAudioContext() {
    if (nixieAudioCtx) return nixieAudioCtx;
    try {
        var Ctx = window.AudioContext || window.webkitAudioContext;
        if (!Ctx) return null;
        nixieAudioCtx = new Ctx();
        return nixieAudioCtx;
    } catch (e) {
        return null;
    }
}

function resumeNixieAudioIfNeeded() {
    var ctx = getNixieAudioContext();
    if (ctx && ctx.state === 'suspended') {
        try { ctx.resume(); } catch (e) {}
    }
}

function isNixieSoundOn() {
    return window.__nixieSoundEnabled === true;
}

function setSoundEnabledFromRust(on, confirmChirp) {
    var prev = window.__nixieSoundEnabled;
    window.__nixieSoundEnabled = !!on;
    syncSoundMenuLabel();
    if (on) resumeNixieAudioIfNeeded();
    if (confirmChirp && on && !prev && soundGateReady && isNixieSoundOn()) {
        playSquareBlip(659, 55, 0.045);
    }
    if (typeof NixieIdlePlay !== 'undefined') NixieIdlePlay.onSoundSettingChanged();
}

function syncSoundMenuLabel() {
    var btn = document.getElementById('menu-sound-toggle');
    if (!btn) return;
    btn.textContent = window.__nixieSoundEnabled ? nixieT('menu.sound_on') : nixieT('menu.sound_off');
}

/** 右键菜单里不随状态变的项（音效/番茄钟另有 sync*） */
function syncMenuStaticLabels() {
    var feed = document.getElementById('menu-feed');
    var folder = document.getElementById('menu-open-folder');
    var petSettings = document.getElementById('menu-pet-settings');
    var about = document.getElementById('menu-about');
    var quit = document.getElementById('menu-quit');
    var aboutTitle = document.getElementById('about-title');
    var aboutClose = document.getElementById('about-close');
    if (feed) feed.textContent = nixieT('menu.feed');
    if (folder) folder.textContent = nixieT('menu.open_folder');
    if (petSettings) petSettings.textContent = nixieT('menu.pet_settings');
    if (about) about.textContent = nixieT('menu.about');
    if (quit) quit.textContent = nixieT('menu.quit');
    if (aboutTitle) aboutTitle.textContent = nixieT('about.title');
    if (aboutClose) aboutClose.textContent = nixieT('about.close');
    syncPetSettingsModalLabels();
}

function getNixiePetSettings() {
    var s = window.__nixiePetSettings;
    if (!s || typeof s !== 'object') return {};
    return s;
}

/** Rust 注入或保存成功后调用：体型缩放 + 界面语言 */
function applyPetSettingsFromRust() {
    var s = getNixiePetSettings();
    var scale = typeof s.bodyScale === 'number' ? s.bodyScale : 1;
    if (scale !== scale || scale <= 0) scale = 1;
    var pet = document.getElementById('pet');
    if (pet) pet.style.setProperty('--pet-body-scale', String(scale));
    var eff = s.effectiveLocale;
    if (eff === 'en' || eff === 'ja' || eff === 'zh') window.__nixieLocale = eff;
    syncMenuStaticLabels();
    syncSoundMenuLabel();
    syncPomoMenuLabel();
    syncWalkMenuLabel();
}

function defaultPetSettingsForm() {
    return { name: 'OINKER_01', bodySize: 'normal', locale: 'zh', breed: 'virtual_pig' };
}

function formValuesFromPetSettings(s) {
    var d = defaultPetSettingsForm();
    if (!s || typeof s !== 'object') return d;
    var bs = s.bodySize;
    if (bs !== 'small' && bs !== 'mini') bs = 'normal';
    var loc = s.locale;
    if (loc !== 'en' && loc !== 'ja' && loc !== 'binary' && loc !== 'zh') loc = d.locale;
    var name = typeof s.name === 'string' ? s.name.trim().slice(0, 64) : d.name;
    if (!name.length) name = d.name;
    var breed = typeof s.breed === 'string' && s.breed.length ? s.breed.slice(0, 48) : d.breed;
    return { name: name, bodySize: bs, locale: loc, breed: breed };
}

function rebuildPetSettingsSelectOptions(extraBreed) {
    var bodySel = document.getElementById('pet-settings-body');
    var locSel = document.getElementById('pet-settings-locale');
    var breedSel = document.getElementById('pet-settings-breed');
    if (!bodySel || !locSel || !breedSel) return;
    function fill(sel, rows) {
        sel.innerHTML = '';
        for (var i = 0; i < rows.length; i++) {
            var o = document.createElement('option');
            o.value = rows[i][0];
            o.textContent = rows[i][1];
            sel.appendChild(o);
        }
    }
    fill(bodySel, [
        ['normal', nixieT('pet.settings.body.normal')],
        ['small', nixieT('pet.settings.body.small')],
        ['mini', nixieT('pet.settings.body.mini')]
    ]);
    fill(locSel, [
        ['zh', nixieT('pet.settings.locale.zh')],
        ['en', nixieT('pet.settings.locale.en')],
        ['ja', nixieT('pet.settings.locale.ja')],
        ['binary', nixieT('pet.settings.locale.binary')]
    ]);
    var breedRows = [['virtual_pig', nixieT('pet.settings.breed.virtual_pig')]];
    if (extraBreed && extraBreed !== 'virtual_pig') breedRows.push([extraBreed, extraBreed]);
    fill(breedSel, breedRows);
}

function syncPetSettingsModalLabels() {
    var title = document.getElementById('pet-settings-title');
    if (title) title.textContent = nixieT('pet.settings.title');
    var nl = document.getElementById('pet-settings-name-label');
    if (nl) nl.textContent = nixieT('pet.settings.name_label');
    var bl = document.getElementById('pet-settings-body-label');
    if (bl) bl.textContent = nixieT('pet.settings.body_label');
    var ll = document.getElementById('pet-settings-locale-label');
    if (ll) ll.textContent = nixieT('pet.settings.locale_label');
    var brl = document.getElementById('pet-settings-breed-label');
    if (brl) brl.textContent = nixieT('pet.settings.breed_label');
    var st = document.getElementById('pet-settings-save-txt');
    if (st) st.textContent = nixieT('pet.settings.save');
    var ct = document.getElementById('pet-settings-cancel-txt');
    if (ct) ct.textContent = nixieT('pet.settings.cancel');
    var rt = document.getElementById('pet-settings-reset-txt');
    if (rt) rt.textContent = nixieT('pet.settings.reset');
    var tagLvl = document.querySelector('.pet-settings-tag-lvl');
    if (tagLvl) tagLvl.textContent = nixieT('pet.settings.tag_lvl');
    var tagMood = document.querySelector('.pet-settings-tag-mood');
    if (tagMood) tagMood.textContent = nixieT('pet.settings.tag_mood');
    var helpBtn = document.getElementById('pet-settings-help');
    if (helpBtn) helpBtn.setAttribute('title', nixieT('pet.settings.help_toast'));
    var gearBtn = document.getElementById('pet-settings-gear');
    if (gearBtn) gearBtn.setAttribute('title', nixieT('menu.open_folder'));
}

function readPetSettingsForm() {
    var nameEl = document.getElementById('pet-settings-name');
    var bodyEl = document.getElementById('pet-settings-body');
    var locEl = document.getElementById('pet-settings-locale');
    var breedEl = document.getElementById('pet-settings-breed');
    var rawName = nameEl ? (nameEl.value || '').trim().slice(0, 64) : '';
    return {
        name: rawName,
        bodySize: bodyEl && bodyEl.value ? bodyEl.value : 'normal',
        locale: locEl && locEl.value ? locEl.value : 'zh',
        breed: breedEl && breedEl.value ? breedEl.value : 'virtual_pig'
    };
}

function writePetSettingsForm(v) {
    var nameEl = document.getElementById('pet-settings-name');
    var bodyEl = document.getElementById('pet-settings-body');
    var locEl = document.getElementById('pet-settings-locale');
    var breedEl = document.getElementById('pet-settings-breed');
    if (nameEl) nameEl.value = v.name;
    if (bodyEl) bodyEl.value = v.bodySize;
    if (locEl) locEl.value = v.locale;
    if (breedEl) breedEl.value = v.breed;
}

function openPetSettingsModal() {
    var aboutSh = document.getElementById('about-sheet');
    if (aboutSh && !aboutSh.hidden) closeAboutSheet();
    var modal = document.getElementById('pet-settings-modal');
    if (!modal) return;
    var cur = formValuesFromPetSettings(getNixiePetSettings());
    rebuildPetSettingsSelectOptions(cur.breed);
    syncPetSettingsModalLabels();
    writePetSettingsForm(cur);
    modal.hidden = false;
    syncPointerPassThroughForPixelMenu(true);
}

function closePetSettingsModal() {
    var modal = document.getElementById('pet-settings-modal');
    if (modal) modal.hidden = true;
    syncPointerPassThroughForPixelMenu(false);
}

function savePetSettingsModal() {
    var v = readPetSettingsForm();
    var payload = JSON.stringify({
        name: v.name,
        bodySize: v.bodySize,
        locale: v.locale,
        breed: v.breed
    });
    try {
        if (window.ipc) window.ipc.postMessage('PET_SETTINGS_SAVE\n' + payload);
    } catch (e) {}
    closePetSettingsModal();
}

/** 方波 + 短包络，偏 8bit / CodePiggy 式 Web 合成 */
function playSquareBlip(freq, durationMs, peakGain, startTime) {
    var ctx = getNixieAudioContext();
    if (!ctx) return;
    var t0 = startTime != null ? startTime : ctx.currentTime;
    var dur = durationMs / 1000;
    var o = ctx.createOscillator();
    var g = ctx.createGain();
    o.type = 'square';
    o.frequency.setValueAtTime(freq, t0);
    g.gain.setValueAtTime(0.0001, t0);
    g.gain.exponentialRampToValueAtTime(Math.max(peakGain, 0.02), t0 + 0.006);
    g.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);
    o.connect(g);
    g.connect(ctx.destination);
    o.start(t0);
    o.stop(t0 + dur + 0.02);
}

function playSquareSequence(notes, stepMs, peakGain) {
    var ctx = getNixieAudioContext();
    if (!ctx) return;
    var t = ctx.currentTime;
    var step = stepMs / 1000;
    for (var i = 0; i < notes.length; i++) {
        playSquareBlip(notes[i], Math.min(stepMs * 0.9, 110), peakGain, t + i * step);
    }
}

/**
 * 自娱自乐「哼歌」专用 8bit：三角波 + 较慢乐句，偏哼鸣感（与方波 mood / Toast 区分）。
 * 仍为零资源 Web Audio，不外链。
 */
function playIdleHumHumCue() {
    if (!isNixieSoundOn()) return;
    resumeNixieAudioIfNeeded();
    var ctx = getNixieAudioContext();
    if (!ctx) return;
    var t0 = ctx.currentTime;
    var peak = 0.048;
    var seq = [
        [440, 125, 0.085],
        [523, 145, 0.08],
        [494, 120, 0.085],
        [587, 155, 0.095],
        [523, 195, 0]
    ];
    var acc = 0;
    for (var i = 0; i < seq.length; i++) {
        var freq = seq[i][0];
        var durMs = seq[i][1];
        var gapAfter = seq[i][2];
        var start = t0 + acc;
        var dur = durMs / 1000;
        var o = ctx.createOscillator();
        var g = ctx.createGain();
        o.type = 'triangle';
        o.frequency.setValueAtTime(freq, start);
        g.gain.setValueAtTime(0.0001, start);
        g.gain.exponentialRampToValueAtTime(Math.max(peak, 0.014), start + 0.022);
        g.gain.exponentialRampToValueAtTime(0.0001, start + dur);
        o.connect(g);
        g.connect(ctx.destination);
        o.start(start);
        o.stop(start + dur + 0.025);
        acc += dur + gapAfter;
    }
}

/**
 * 自娱自乐闲置动作 8bit（与 REGISTRY id 对齐）；仅当用户开启小猪音效且 ctx.soundOn 时由 onStart 调用。
 */
function playIdlePlayCue(id) {
    if (!isNixieSoundOn()) return;
    resumeNixieAudioIfNeeded();
    switch (id) {
        case 'hum':
            playIdleHumHumCue();
            return;
        case 'spin':
            /* 转圈：一圈上扬琶音 */
            playSquareSequence([523, 659, 784, 1046, 1318], 46, 0.054);
            return;
        case 'peek':
            /* 张望：左右探头 */
            playSquareSequence([659, 523, 698, 523], 68, 0.05);
            return;
        case 'nod':
            /* 点头：两下轻敲 */
            playSquareSequence([392, 349], 100, 0.042);
            return;
        case 'morning':
            playSquareSequence([523, 659, 784], 72, 0.046);
            return;
        case 'evening':
            playSquareSequence([784, 659, 523], 74, 0.046);
            return;
        case 'weekend':
            playSquareSequence([523, 659, 523, 784], 58, 0.048);
            return;
        case 'late':
            playSquareSequence([311, 349, 392], 88, 0.038);
            return;
        default:
            return;
    }
}

function playMoodCue(mood) {
    if (!isNixieSoundOn()) return;
    resumeNixieAudioIfNeeded();
    var m = mood || 'idle';
    if (m === 'idle' || m === 'coding') {
        playSquareSequence([523, 659], 70, 0.06);
    } else if (m === 'thinking') {
        playSquareSequence([392, 523, 659], 55, 0.055);
    } else if (m === 'writing') {
        playSquareSequence([659, 698, 784], 45, 0.05);
    } else if (m === 'running') {
        var ctx = getNixieAudioContext();
        if (!ctx) return;
        var t0 = ctx.currentTime;
        playSquareBlip(220, 90, 0.07, t0);
        playSquareBlip(196, 90, 0.06, t0 + 0.1);
    } else if (m === 'searching') {
        playSquareSequence([523, 440, 523], 60, 0.055);
    } else if (m === 'web-search') {
        playSquareSequence([392, 523, 659, 784], 50, 0.05);
    } else if (m === 'error') {
        playSquareSequence([196, 185, 175], 80, 0.08);
    } else if (m === 'success') {
        playTaskSuccessTwoSyllableCue('s');
    } else if (m === 'sleeping') {
        playSquareBlip(330, 140, 0.045);
    } else {
        playSquareBlip(440, 80, 0.05);
    }
}

/**
 * 任务完成（成功庆祝）：双音节「短促 + 休止 + 高八度长音」，与琶音类 mood / Toast 单音拉开辨识度。
 * tier：xs/s/m/l 用不同八度根音 + 第二音略加长；xs 更短更亮表示「秒过」。
 */
function playTaskSuccessTwoSyllableCue(tier) {
    if (!isNixieSoundOn()) return;
    resumeNixieAudioIfNeeded();
    var ctx = getNixieAudioContext();
    if (!ctx) return;
    var t = tier || 's';
    var f1;
    var f2;
    var gapMs;
    var dur1;
    var dur2;
    var peak;
    if (t === 'l') {
        f1 = 523;
        f2 = 1046;
        gapMs = 88;
        dur1 = 52;
        dur2 = 280;
        peak = 0.063;
    } else if (t === 'm') {
        f1 = 587;
        f2 = 1175;
        gapMs = 84;
        dur1 = 50;
        dur2 = 235;
        peak = 0.058;
    } else if (t === 'xs') {
        f1 = 784;
        f2 = 1568;
        gapMs = 68;
        dur1 = 36;
        dur2 = 148;
        peak = 0.047;
    } else {
        f1 = 659;
        f2 = 1318;
        gapMs = 80;
        dur1 = 46;
        dur2 = 205;
        peak = 0.054;
    }
    var t0 = ctx.currentTime;
    playSquareBlip(f1, dur1, peak, t0);
    playSquareBlip(f2, dur2, peak * 0.96, t0 + dur1 / 1000 + gapMs / 1000);
}

function playCelebrationCue(tier, isError) {
    if (!isNixieSoundOn()) return;
    resumeNixieAudioIfNeeded();
    var t = tier || 's';
    if (isError) {
        if (t === 'l') {
            playSquareSequence([147, 139, 131, 123], 100, 0.09);
        } else if (t === 'm') {
            playSquareSequence([165, 155, 147], 85, 0.085);
        } else {
            playSquareSequence([196, 185], 90, 0.08);
        }
    } else {
        playTaskSuccessTwoSyllableCue(t);
    }
}

function playToastCue() {
    if (!isNixieSoundOn()) return;
    resumeNixieAudioIfNeeded();
    playSquareBlip(880, 45, 0.04);
}

function playFeedCue() {
    if (!isNixieSoundOn()) return;
    resumeNixieAudioIfNeeded();
    playSquareSequence([659, 784, 988], 65, 0.07);
}

/** 启动跑入 `#pet.pet-enter-run`：四音上扬（不依赖 soundGate，与入场同帧） */
function playPetEnterCue() {
    if (!isNixieSoundOn()) return;
    resumeNixieAudioIfNeeded();
    playSquareSequence([523, 659, 784, 1046], 56, 0.052);
}

/** 安全退出 `#pet.pet-exit-flee`：四音下行告别 */
function playPetExitCue() {
    if (!isNixieSoundOn()) return;
    resumeNixieAudioIfNeeded();
    playSquareSequence([880, 659, 523, 392], 58, 0.048);
}

/** 逗小猪：轻反馈；comfort 用于 Error 等负向 mood */
function playPokeCue(comfort) {
    if (!isNixieSoundOn()) return;
    resumeNixieAudioIfNeeded();
    if (comfort) {
        playSquareBlip(523, 50, 0.028);
    } else {
        playSquareBlip(988, 42, 0.032);
    }
}

var petPokeDown = null;
var petPokeDragSent = false;
var POKE_DRAG_THRESHOLD_PX = 9;
var POKE_THROTTLE_MS = 420;
var lastPokeAt = 0;
var pokeAsideTimer = null;
var pokeMainTimer = null;
var pokeMainPrevText = '';

function clearPokeAsideTimer() {
    if (pokeAsideTimer) {
        clearTimeout(pokeAsideTimer);
        pokeAsideTimer = null;
    }
}

function clearPokeMainTimer() {
    if (pokeMainTimer) {
        clearTimeout(pokeMainTimer);
        pokeMainTimer = null;
    }
    pokeMainPrevText = '';
}

/**
 * 清逗猪定时器 + 与 Toast/哒哒哒 共用的 bubbleHideTimer，并收起气泡。
 * 「哒哒哒」来自 Overlay UserTypingPulse（native 打字脉冲，台词约 5%），不是 idle 台词；
 * 逗猪若清掉 toast 的 hide 定时器却不恢复，会把旧主行（含哒哒哒）锁在屏上。
 */
function clearPokeBubbleTimers() {
    clearPokeAsideTimer();
    clearPokeMainTimer();
    if (typeof bubbleHideTimer !== 'undefined' && bubbleHideTimer) {
        clearTimeout(bubbleHideTimer);
        bubbleHideTimer = null;
    }
    var bubble = document.getElementById('bubble');
    if (bubble) bubble.classList.remove('bubble-visible');
}

/**
 * 纯闲置：主行只有 mood 台词、无焦点/子任务副行 → 逗猪短句走主行（#mood-text），更直观。
 * 有子任务副标题或焦点文件等「任务向」信息 → 走副行，避免盖住主状态。
 */
function isPokePlainIdleContext() {
    var mood = getPetMoodClass();
    if (mood !== 'idle' && mood !== 'sleeping') return false;
    var sub = document.getElementById('bubble-subtitle');
    var ff = document.getElementById('focus-file-hint');
    if (sub && String(sub.textContent || '').trim().length > 0) return false;
    if (ff && String(ff.textContent || '').trim().length > 0) return false;
    return true;
}

/** 主行临时替换；updateMood 会清定时器并以 Rust 台词为准 */
function showPokeMainLine(text, durationMs) {
    clearPokeBubbleTimers();
    var mt = document.getElementById('mood-text');
    if (!mt) return;
    pokeMainPrevText = mt.textContent;
    mt.textContent = text;
    var bubble = document.getElementById('bubble');
    if (bubble) bubble.classList.add('bubble-visible');
    var ms = durationMs != null ? durationMs : 2200;
    pokeMainTimer = setTimeout(function() {
        pokeMainTimer = null;
        var mt2 = document.getElementById('mood-text');
        if (mt2) mt2.textContent = pokeMainPrevText;
        pokeMainPrevText = '';
        var b = document.getElementById('bubble');
        if (bubbleHideTimer) {
            clearTimeout(bubbleHideTimer);
            bubbleHideTimer = null;
        }
        bubbleHideTimer = setTimeout(function() {
            if (b) b.classList.remove('bubble-visible');
            bubbleHideTimer = null;
        }, 2500);
    }, ms);
}

/** 用气泡副行展示短句；副行已有子任务等文案时跳过 */
function showPokeAsideLine(text, durationMs) {
    var sub = document.getElementById('bubble-subtitle');
    if (!sub) return;
    if (sub.textContent && String(sub.textContent).trim().length > 0) return;
    clearPokeBubbleTimers();
    sub.textContent = text;
    sub.setAttribute('aria-hidden', 'false');
    var bubble = document.getElementById('bubble');
    if (bubble) bubble.classList.add('bubble-visible');
    var ms = durationMs != null ? durationMs : 2200;
    pokeAsideTimer = setTimeout(function() {
        pokeAsideTimer = null;
        sub.textContent = '';
        sub.setAttribute('aria-hidden', 'true');
    }, ms);
}

function showPokeLineSmart(text, durationMs) {
    if (isPokePlainIdleContext()) {
        showPokeMainLine(text, durationMs);
    } else {
        showPokeAsideLine(text, durationMs);
    }
}

function getPetMoodClass() {
    var pet = document.getElementById('pet');
    if (!pet) return 'idle';
    var m = (pet.className || '').match(/\bmood-([\w-]+)\b/);
    return m ? m[1] : 'idle';
}

function runPetPokeFeedback() {
    var now = Date.now();
    if (now - lastPokeAt < POKE_THROTTLE_MS) return;
    lastPokeAt = now;

    clearPokeBubbleTimers();

    try {
        window.ipc.postMessage('poke');
    } catch (e) {}

    if (typeof NixieIdlePlay !== 'undefined') NixieIdlePlay.cancel('poke');

    var pet = document.getElementById('pet');
    if (!pet) return;

    var mood = getPetMoodClass();
    var wasSleeping = mood === 'sleeping';
    var isComfort = mood === 'error';

    pet.classList.remove('pet-poke');
    void pet.offsetWidth;
    pet.classList.add('pet-poke');
    if (isComfort) {
        pet.classList.add('pet-poke-comfort');
    } else {
        pet.classList.add('pet-jump');
    }

    if (soundGateReady) playPokeCue(isComfort);

    if (!isComfort) {
        if (wasSleeping) {
            if (Math.random() < 0.5) {
                showPokeLineSmart(nixiePickLine('poke.wake'), 2600);
            }
        } else if (Math.random() < 0.2) {
            showPokeLineSmart(nixiePickLine('poke.bubble'), 2000);
        }
    }

    setTimeout(function() {
        pet.classList.remove('pet-jump', 'pet-poke-comfort', 'pet-poke');
    }, isComfort ? 520 : 380);
}

function onPetPokePointerMove(e) {
    if (!petPokeDown || e.buttons !== 1) return;
    var dx = e.clientX - petPokeDown.x;
    var dy = e.clientY - petPokeDown.y;
    if (!petPokeDragSent && dx * dx + dy * dy > POKE_DRAG_THRESHOLD_PX * POKE_DRAG_THRESHOLD_PX) {
        petPokeDragSent = true;
        try {
            window.ipc.postMessage('drag');
        } catch (err) {}
    }
}

function onPetPokePointerUp(e) {
    if (!petPokeDown) return;
    var dx = e.clientX - petPokeDown.x;
    var dy = e.clientY - petPokeDown.y;
    var dist = Math.sqrt(dx * dx + dy * dy);
    var dt = Date.now() - petPokeDown.t;
    var startedOnPet = petPokeDown.onPet;
    petPokeDown = null;
    var dragSent = petPokeDragSent;
    petPokeDragSent = false;
    if (dragSent) return;
    if (!startedOnPet) return;
    if (dist >= POKE_DRAG_THRESHOLD_PX || dt > 700) return;
    runPetPokeFeedback();
}

/** Idle Play：自娱自乐调度器 + 可注册动作（与 PetMood / Rust 解耦） */
var NixieIdlePlay = (function() {
    var ARM_MIN_MS = 45000;
    var ARM_MAX_MS = 120000;
    var armTimer = null;
    var endTimer = null;
    var playingId = null;
    var idlePlayLineActive = false;
    var idlePlayLinePrevText = '';

    /** 气泡主行：自娱自乐台词；cancel / 自然结束 / updateMood 前会恢复 */
    function beginIdlePlayLine(text) {
        endIdlePlayLine();
        var mt = document.getElementById('mood-text');
        if (!mt) return;
        idlePlayLinePrevText = mt.textContent;
        idlePlayLineActive = true;
        mt.textContent = text;
        var bubble = document.getElementById('bubble');
        if (bubble) bubble.classList.add('bubble-visible');
        if (typeof bubbleHideTimer !== 'undefined' && bubbleHideTimer) {
            clearTimeout(bubbleHideTimer);
            bubbleHideTimer = null;
        }
    }

    function endIdlePlayLine() {
        if (!idlePlayLineActive) return;
        idlePlayLineActive = false;
        var mt = document.getElementById('mood-text');
        if (mt) mt.textContent = idlePlayLinePrevText;
        idlePlayLinePrevText = '';
        var bubble = document.getElementById('bubble');
        if (bubble) bubble.classList.remove('bubble-visible');
    }

    function getPet() {
        return document.getElementById('pet');
    }

    function stripIdlePlayClasses(pet) {
        if (!pet || !pet.classList) return;
        var toRemove = [];
        for (var i = 0; i < pet.classList.length; i++) {
            var c = pet.classList[i];
            if (c.indexOf('idle-play-') === 0) toRemove.push(c);
        }
        for (var j = 0; j < toRemove.length; j++) pet.classList.remove(toRemove[j]);
        pet.removeAttribute('data-idle-play');
    }

    function buildContext() {
        var pet = getPet();
        if (!pet) return null;
        var cls = pet.className || '';
        var m = cls.match(/\bmood-([\w-]+)\b/);
        var mood = m ? m[1] : 'idle';
        var d = new Date();
        var hour = d.getHours();
        var minute = d.getMinutes();
        var dow = d.getDay();
        var isWeekend = dow === 0 || dow === 6;
        var wph = pet.getAttribute('data-walk-phase') || 'off';
        return {
            mood: mood,
            isIdle: mood === 'idle',
            isSleeping: mood === 'sleeping',
            soundOn: typeof isNixieSoundOn === 'function' && isNixieSoundOn(),
            walkFollowing: wph === 'following',
            walkHoverIntent: wph === 'hover_intent',
            pomoRunning: pet.getAttribute('data-pomo-running') === '1',
            celebration: !!pet.getAttribute('data-celebration-tier'),
            pageHidden: document.hidden,
            hour: hour,
            minute: minute,
            dayOfWeek: dow,
            isWeekend: isWeekend,
            isWeekday: !isWeekend,
            minuteOfDay: hour * 60 + minute
        };
    }

    /** [hourStart, hourEnd) 整点小时；若 start > end 则跨午夜 */
    function hourInRange(h, start, end) {
        if (start === end) return false;
        if (start < end) return h >= start && h < end;
        return h >= start || h < end;
    }

    function clockToMinutes(cl) {
        if (!cl) return 0;
        var h = cl.hour != null ? cl.hour : 0;
        var m = cl.minute != null ? cl.minute : 0;
        return h * 60 + m;
    }

    /** [startM, endM) 分钟（0–1440）；若 startM > endM 则跨午夜 */
    function minuteOfDayInRange(nowM, startM, endM) {
        if (startM === endM) return false;
        if (startM < endM) return nowM >= startM && nowM < endM;
        return nowM >= startM || nowM < endM;
    }

    /**
     * @param {Object} tf
     * @param {boolean} [tf.weekdaysOnly]
     * @param {boolean} [tf.weekendsOnly]
     * @param {{hour:number, minute?:number}} [tf.startClock] — 与 endClock 搭配，精确到分，区间 [start, end)
     * @param {{hour:number, minute?:number}} [tf.endClock]
     * @param {number} [tf.hourStart] — 仅整点时段（兼容旧写法）
     * @param {number} [tf.hourEnd]
     */
    function matchesTimeFilter(tf, ctx) {
        if (!tf) return true;
        if (tf.weekdaysOnly && ctx.isWeekend) return false;
        if (tf.weekendsOnly && !ctx.isWeekend) return false;
        if (tf.startClock && tf.endClock) {
            var startM = clockToMinutes(tf.startClock);
            var endM = clockToMinutes(tf.endClock);
            var nowM = ctx.minuteOfDay != null ? ctx.minuteOfDay : ctx.hour * 60 + ctx.minute;
            if (!minuteOfDayInRange(nowM, startM, endM)) return false;
        } else if (tf.hourStart != null && tf.hourEnd != null) {
            if (!hourInRange(ctx.hour, tf.hourStart, tf.hourEnd)) return false;
        }
        return true;
    }

    function canSchedule(ctx) {
        if (!ctx) return false;
        if (ctx.pageHidden) return false;
        if (ctx.walkFollowing || ctx.walkHoverIntent || ctx.pomoRunning || ctx.celebration) return false;
        if (!ctx.isIdle && !ctx.isSleeping) return false;
        return true;
    }

    /**
     * @typedef {Object} IdlePlayAction
     * @property {string} id
     * @property {number} [weight]
     * @property {string[]} [moods] — 允许出现的 mood class（不含 mood- 前缀）
     * @property {boolean} [requireSound] — 为 true 时仅当用户开启小猪音效才参与抽取
     * @property {boolean} [requireSleeping] — 为 true 时仅 sleeping（预留：梦游等）
     * @property {boolean} [requireIdle] — 为 true 时仅 idle（预留）
     * @property {number} durationMs
     * @property {string[]} [lines] — 随机一条显示在气泡主行
     * @property {Object} [timeFilter] — 本地时间；优先 startClock/endClock（约 30min～1.5h 窄窗），否则 hourStart/hourEnd 整点
     */
    var REGISTRY = [];
    var registryInited = false;

    function register(a) {
        REGISTRY.push(a);
    }

    function matchesAction(a, ctx) {
        if (a.requireSleeping === true && !ctx.isSleeping) return false;
        if (a.requireIdle === true && !ctx.isIdle) return false;
        if (a.moods && a.moods.indexOf(ctx.mood) === -1) return false;
        if (a.requireSound && !ctx.soundOn) return false;
        if (a.timeFilter && !matchesTimeFilter(a.timeFilter, ctx)) return false;
        return true;
    }

    function pickAction(ctx) {
        var candidates = [];
        var wsum = 0;
        for (var i = 0; i < REGISTRY.length; i++) {
            var a = REGISTRY[i];
            if (!matchesAction(a, ctx)) continue;
            var w = a.weight != null ? a.weight : 1;
            if (w <= 0) continue;
            candidates.push({ a: a, w: w });
            wsum += w;
        }
        if (wsum <= 0 || candidates.length === 0) return null;
        var r = Math.random() * wsum;
        for (var j = 0; j < candidates.length; j++) {
            r -= candidates[j].w;
            if (r <= 0) return candidates[j].a;
        }
        return candidates[candidates.length - 1].a;
    }

    function clearTimers() {
        if (armTimer != null) {
            clearTimeout(armTimer);
            armTimer = null;
        }
        if (endTimer != null) {
            clearTimeout(endTimer);
            endTimer = null;
        }
    }

    function cancel(reason) {
        clearTimers();
        playingId = null;
        endIdlePlayLine();
        stripIdlePlayClasses(getPet());
        scheduleArm();
    }

    function scheduleArm() {
        clearTimers();
        if (playingId != null) return;
        var ctx = buildContext();
        if (!canSchedule(ctx)) return;
        var delay = ARM_MIN_MS + Math.random() * (ARM_MAX_MS - ARM_MIN_MS);
        armTimer = setTimeout(function() {
            armTimer = null;
            var c = buildContext();
            if (!canSchedule(c)) {
                scheduleArm();
                return;
            }
            var action = pickAction(c);
            if (!action) {
                scheduleArm();
                return;
            }
            var pet = getPet();
            if (!pet) return;
            playingId = action.id;
            pet.setAttribute('data-idle-play', action.id);
            pet.classList.add('idle-play-' + action.id);
            var lines = action.lines;
            if (lines && lines.length) {
                beginIdlePlayLine(lines[Math.floor(Math.random() * lines.length)]);
            }
            if (typeof action.onStart === 'function') {
                try {
                    action.onStart(c);
                } catch (e) {}
            }
            var dur = action.durationMs != null ? action.durationMs : 2000;
            endTimer = setTimeout(function() {
                endTimer = null;
                playingId = null;
                endIdlePlayLine();
                stripIdlePlayClasses(pet);
                if (typeof action.onEnd === 'function') {
                    try {
                        action.onEnd();
                    } catch (e2) {}
                }
                scheduleArm();
            }, dur);
        }, delay);
    }

    function onAfterMoodUpdate(mood) {
        scheduleArm();
    }

    function onSoundSettingChanged() {
        cancel('sound');
    }

    function init() {
        if (registryInited) return;
        registryInited = true;
        register({
            id: 'peek',
            weight: 1.2,
            moods: ['idle', 'sleeping'],
            durationMs: 4200,
            lines: [
                '咦？那边好像有动静～',
                '让我看看谁在偷偷写代码～',
                '左瞧瞧，右瞧瞧～',
                '（探头）应该没人发现我吧？'
            ],
            onStart: function(ctx) {
                if (!ctx.soundOn) return;
                playIdlePlayCue('peek');
            }
        });
        register({
            id: 'spin',
            weight: 1,
            moods: ['idle'],
            durationMs: 1200,
            lines: [
                '转呀转，开心转圈圈～',
                '哼唧，今天也要加油鸭～',
                '头晕晕～但是好快乐～',
                '我是小旋风猪猪～'
            ],
            onStart: function(ctx) {
                if (!ctx.soundOn) return;
                playIdlePlayCue('spin');
            }
        });
        register({
            id: 'nod',
            weight: 1,
            moods: ['idle', 'sleeping'],
            durationMs: 2000,
            lines: [
                '霍霍……好多好吃的……',
                'zzZ……再睡一会儿嘛……',
                '嗯嗯……梦到罐罐了……',
                '脑袋好重……就点一下下～'
            ],
            onStart: function(ctx) {
                if (!ctx.soundOn) return;
                playIdlePlayCue('nod');
            }
        });
        register({
            id: 'hum',
            weight: 0.85,
            moods: ['idle', 'sleeping'],
            requireSound: true,
            durationMs: 2400,
            lines: [
                '哼～哼哼～♪',
                '啦啦啦～心情不错～',
                '送给认真工作的你一小段～',
                '嘟嘟～噜～（小声哼）'
            ],
            onStart: function(ctx) {
                if (!ctx.soundOn) return;
                playIdlePlayCue('hum');
            }
        });
        /* 时间向：各约 90min 窄窗 [start,end)；动效见 nyanpig.css idle-play-{morning,evening,weekend,late} */
        register({
            id: 'morning',
            weight: 0.55,
            moods: ['idle', 'sleeping'],
            durationMs: 2600,
            timeFilter: {
                weekdaysOnly: true,
                startClock: { hour: 8, minute: 30 },
                endClock: { hour: 10, minute: 0 }
            },
            lines: [
                '八点半后的懒腰最香了～',
                '伸伸胳膊……大脑开机中～',
                '阳光刚好，适合伸个懒腰～',
                '唔……再伸一下就去干活～'
            ],
            onStart: function(ctx) {
                if (!ctx.soundOn) return;
                playIdlePlayCue('morning');
            }
        });
        register({
            id: 'evening',
            weight: 0.55,
            moods: ['idle', 'sleeping'],
            durationMs: 2800,
            timeFilter: {
                startClock: { hour: 18, minute: 0 },
                endClock: { hour: 19, minute: 30 }
            },
            lines: [
                '这会儿天光刚刚好～',
                '傍晚的风……想多待一分钟～',
                '窗外颜色变温柔了～',
                '收工前的最后一抹亮～'
            ],
            onStart: function(ctx) {
                if (!ctx.soundOn) return;
                playIdlePlayCue('evening');
            }
        });
        register({
            id: 'weekend',
            weight: 0.5,
            moods: ['idle', 'sleeping'],
            durationMs: 3000,
            timeFilter: {
                weekendsOnly: true,
                startClock: { hour: 10, minute: 30 },
                endClock: { hour: 12, minute: 0 }
            },
            lines: [
                '周末十点半……赖床合法～',
                '慢吞吞才是周末正义～',
                '不 rush，先伸个懒腰～',
                '早午餐之间的发呆时间～'
            ],
            onStart: function(ctx) {
                if (!ctx.soundOn) return;
                playIdlePlayCue('weekend');
            }
        });
        register({
            id: 'late',
            weight: 0.42,
            moods: ['idle', 'sleeping'],
            durationMs: 3200,
            timeFilter: {
                startClock: { hour: 23, minute: 30 },
                endClock: { hour: 1, minute: 0 }
            },
            lines: [
                '十一点半了……还在卷？我陪你～',
                '这个点的屏幕光……有点浪漫～',
                '星星值班中～',
                '月亮不睡我不睡～（小声）'
            ],
            onStart: function(ctx) {
                if (!ctx.soundOn) return;
                playIdlePlayCue('late');
            }
        });
        document.addEventListener('visibilitychange', function() {
            if (document.hidden) cancel('hidden');
            else scheduleArm();
        });
    }

    return {
        init: init,
        cancel: cancel,
        onAfterMoodUpdate: onAfterMoodUpdate,
        onSoundSettingChanged: onSoundSettingChanged,
        scheduleArm: scheduleArm,
        /** 扩展新动作（梦游、梦话等）：需在 init() 之后调用 */
        registerAction: register
    };
})();

// 气泡与当前状态绑定：切换状态时旧气泡消失、显示新状态台词并重新计时 2.5s，故不一定会显示满 2.5s
function setFocusFileHint(name) {
    var el = document.getElementById('focus-file-hint');
    if (!el) return;
    var s = name != null && String(name).length ? String(name) : '';
    el.textContent = s;
}
/** Phase 3：子任务等副标题（与焦点文件名分行） */
function setBubbleSubtitle(text) {
    var el = document.getElementById('bubble-subtitle');
    if (!el) return;
    var s = text != null && String(text).length ? String(text) : '';
    el.textContent = s;
}
/** mood 未变时仅刷新次要行 */
function updateBubbleSecondary(focusFile, subtitle) {
    setFocusFileHint(focusFile);
    setBubbleSubtitle(subtitle);
}
function updateMood(mood, label, hasExt, quote, focusFile, subtitle, skipSound) {
    if (nixieBootEntranceActive) {
        nixieEntranceDeferred = {
            kind: 'mood',
            mood: mood,
            label: label,
            hasExt: hasExt,
            quote: quote,
            focusFile: focusFile,
            subtitle: subtitle,
            skipSound: !!skipSound
        };
        return;
    }
    if (typeof NixieIdlePlay !== 'undefined') NixieIdlePlay.cancel('mood');
    clearPokeBubbleTimers();
    var pet = document.getElementById('pet');
    pet.className = 'mood-' + mood;
    if (mood !== 'success') {
        pet.removeAttribute('data-success-trivial');
    }
    document.getElementById('mood-text').textContent = quote !== undefined ? quote : label;
    if (focusFile !== undefined) {
        setFocusFileHint(focusFile);
    }
    if (subtitle !== undefined) {
        setBubbleSubtitle(subtitle);
    }
    var dot = document.getElementById('ext-dot');
    if (hasExt) dot.classList.add('active');
    else dot.classList.remove('active');

    var bubble = document.getElementById('bubble');
    if (bubbleHideTimer) clearTimeout(bubbleHideTimer);
    bubble.classList.add('bubble-visible');
    bubbleHideTimer = setTimeout(function() {
        bubble.classList.remove('bubble-visible');
        bubbleHideTimer = null;
    }, 2500);
    if (!skipSound && soundGateReady && isNixieSoundOn()) {
        playMoodCue(mood);
    }
    if (typeof NixieIdlePlay !== 'undefined') NixieIdlePlay.onAfterMoodUpdate(mood);
}
/** 本地 git 分支切换 / 同分支新提交等：由 Rust 在快照变化时推送，不占心情台词位 */
function showGitTip(msg) {
    clearPokeBubbleTimers();
    var bubble = document.getElementById('bubble');
    document.getElementById('mood-text').textContent = msg || '';
    setBubbleSubtitle('');
    if (bubbleHideTimer) clearTimeout(bubbleHideTimer);
    bubble.classList.add('bubble-visible');
    bubbleHideTimer = setTimeout(function() {
        bubble.classList.remove('bubble-visible');
        bubbleHideTimer = null;
    }, 2800);
}
// 工具成功等一次性提示：当前形态下气泡展示文案并跳跃一下（不切换状态）
function showToast(msg, skipToastSound) {
    clearPokeBubbleTimers();
    var pet = document.getElementById('pet');
    var bubble = document.getElementById('bubble');
    document.getElementById('mood-text').textContent = msg || nixieT('toast.exec_success');
    if (bubbleHideTimer) clearTimeout(bubbleHideTimer);
    bubble.classList.add('bubble-visible');
    pet.classList.add('pet-jump');
    setTimeout(function() { pet.classList.remove('pet-jump'); }, 320);
    bubbleHideTimer = setTimeout(function() {
        bubble.classList.remove('bubble-visible');
        bubbleHideTimer = null;
    }, 2500);
    if (!skipToastSound && soundGateReady && isNixieSoundOn()) playToastCue();
}
/** 打字脉冲：feedback 为真时跳跃 + Toast 音效（每 3 次脉冲一次）；showLine 为真时展示「哒哒哒」气泡（约 5%）。 */
function showUserTypingPulse(showLine, feedback) {
    var pet = document.getElementById('pet');
    var bubble = document.getElementById('bubble');
    if (showLine) {
        clearPokeBubbleTimers();
        document.getElementById('mood-text').textContent = nixieT('toast.user_typing');
        if (bubbleHideTimer) clearTimeout(bubbleHideTimer);
        bubble.classList.add('bubble-visible');
        bubbleHideTimer = setTimeout(function() {
            bubble.classList.remove('bubble-visible');
            bubbleHideTimer = null;
        }, 2500);
    }
    if (feedback) {
        pet.classList.add('pet-jump');
        setTimeout(function() { pet.classList.remove('pet-jump'); }, 320);
        if (soundGateReady && isNixieSoundOn()) playToastCue();
    }
}
/** 同一次脚本里先切 mood 再挂庆祝，避免 WKWebView 在两段 evaluate 之间绘制一帧「仍是 writing 皮 + 庆祝层」 */
function updateMoodThenApplyCelebration(mood, label, hasExt, quote, focusFile, subtitle, tier, durationMs, isError) {
    if (nixieBootEntranceActive) {
        nixieEntranceDeferred = {
            kind: 'celebration',
            mood: mood,
            label: label,
            hasExt: hasExt,
            quote: quote,
            focusFile: focusFile,
            subtitle: subtitle,
            tier: tier,
            durationMs: durationMs,
            isError: isError
        };
        return;
    }
    updateMood(mood, label, hasExt, quote, focusFile, subtitle, true);
    applyCelebrationTier(tier, durationMs, isError);
}
/** Overlay：庆祝分档（与 Core mood 独立；未来 AnimalRenderer 可接不同动效） */
function applyCelebrationTier(tier, durationMs, isError) {
    if (typeof NixieIdlePlay !== 'undefined') NixieIdlePlay.cancel('celebration');
    var pet = document.getElementById('pet');
    if (celebrationPauseTimer) {
        clearTimeout(celebrationPauseTimer);
        celebrationPauseTimer = null;
    }
    celebrationMotionEpoch++;
    var motionEpoch = celebrationMotionEpoch;
    pet.removeAttribute('data-celebration-pause-motion');
    pet.setAttribute('data-celebration-tier', tier || '');
    pet.setAttribute('data-celebration-ms', String(durationMs || 0));
    pet.setAttribute('data-celebration-err', isError ? '1' : '0');
    if (!isError && tier === 'xs') {
        pet.setAttribute('data-success-trivial', '1');
    } else {
        pet.removeAttribute('data-success-trivial');
    }
    celebrationPauseTimer = setTimeout(function() {
        celebrationPauseTimer = null;
        if (motionEpoch !== celebrationMotionEpoch) return;
        if (!pet.getAttribute('data-celebration-tier')) return;
        if (window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
            return;
        }
        pet.setAttribute('data-celebration-pause-motion', '1');
        wireCelebrationPauseRelease(pet, motionEpoch);
    }, CELEBRATION_PAUSE_MS);
    if (celebrationClearTimer) clearTimeout(celebrationClearTimer);
    celebrationClearTimer = setTimeout(function() {
        celebrationMotionEpoch++;
        if (celebrationPauseTimer) {
            clearTimeout(celebrationPauseTimer);
            celebrationPauseTimer = null;
        }
        pet.removeAttribute('data-celebration-tier');
        pet.removeAttribute('data-celebration-ms');
        pet.removeAttribute('data-celebration-err');
        pet.removeAttribute('data-celebration-pause-motion');
        celebrationClearTimer = null;
    }, CELEBRATION_ATTR_HOLD_MS);
    if (soundGateReady && isNixieSoundOn()) playCelebrationCue(tier, isError);
}
function setFeedAvailable(can) {
    var pet = document.getElementById('pet');
    pet.setAttribute('data-can-feed', can ? '1' : '0');
}
function syncWalkMenuLabel() {
    var btn = document.getElementById('menu-walk-toggle');
    if (!btn) return;
    if (window.__nixieWalkSupported !== true) {
        btn.textContent = nixieT('menu.walk_unsupported');
        btn.disabled = true;
        return;
    }
    btn.disabled = false;
    var pet = document.getElementById('pet');
    var phase = pet ? pet.getAttribute('data-walk-phase') || 'off' : 'off';
    var on = phase !== 'off';
    btn.textContent = on ? nixieT('menu.walk_on') : nixieT('menu.walk_off');
}

function playWalkStartCue() {
    if (!isNixieSoundOn()) return;
    resumeNixieAudioIfNeeded();
    playSquareSequence([523, 659, 784], 88, 0.065);
}

function playWalkEndCue() {
    if (!isNixieSoundOn()) return;
    resumeNixieAudioIfNeeded();
    playSquareSequence([784, 659, 523], 85, 0.058);
}

function walkFeedbackFromPhaseChange(prev, next) {
    if (prev !== 'following' && next === 'following') {
        if (soundGateReady && isNixieSoundOn()) playWalkStartCue();
        var line = typeof nixiePickLine === 'function' ? nixiePickLine('walk.start') : '';
        if (line) showPokeLineSmart(line, 2400);
        return;
    }
    if (prev === 'following' && next !== 'following') {
        if (soundGateReady && isNixieSoundOn()) playWalkEndCue();
        var lineEnd = typeof nixiePickLine === 'function' ? nixiePickLine('walk.end') : '';
        if (lineEnd) showPokeLineSmart(lineEnd, 2200);
    }
}

/** 遛猪 UI 阶段（Rust 仅 off/idle/following；hover_intent 仅前端蓄力） */
function applyWalkPhaseAttr(phase) {
    var pet = document.getElementById('pet');
    if (!pet) return;
    var prevWalk = pet.getAttribute('data-walk-phase') || 'off';
    if (phase !== 'hover_intent') walkClearHoverTimer();
    if (prevWalk === 'following' && phase !== 'following') {
        lastWalkChasePosted = null;
        try {
            window.ipc.postMessage('walk_chase_0');
        } catch (e) {}
    }
    if (phase === 'following') {
        lastWalkChasePosted = null;
    }
    pet.setAttribute('data-walk-phase', phase || 'off');
    if (typeof NixieIdlePlay !== 'undefined') {
        if (phase === 'following') NixieIdlePlay.cancel('walk');
        else if (phase === 'hover_intent') NixieIdlePlay.cancel('walk');
        else NixieIdlePlay.scheduleArm();
    }
    syncWalkMenuLabel();
}
function setWalkPhase(phase) {
    var pet = document.getElementById('pet');
    var prev = pet ? pet.getAttribute('data-walk-phase') || 'off' : 'off';
    var next = phase || 'off';
    walkFeedbackFromPhaseChange(prev, next);
    applyWalkPhaseAttr(next);
}

var WALK_HOVER_MS = 7000;

var walkHoverTimer = null;

function walkClearHoverTimer() {
    if (walkHoverTimer) {
        clearTimeout(walkHoverTimer);
        walkHoverTimer = null;
    }
}

function walkOnMenuOpenResetHover() {
    var pet = document.getElementById('pet');
    if (!pet) return;
    if (pet.getAttribute('data-walk-phase') === 'hover_intent') {
        applyWalkPhaseAttr('idle');
    }
}

function walkOnPetEnter() {
    if (window.__nixieWalkSupported !== true) return;
    var pet = document.getElementById('pet');
    if (!pet) return;
    if (pet.getAttribute('data-walk-phase') !== 'idle') return;
    walkClearHoverTimer();
    applyWalkPhaseAttr('hover_intent');
    walkHoverTimer = setTimeout(function() {
        walkHoverTimer = null;
        var p = document.getElementById('pet');
        if (!p || p.getAttribute('data-walk-phase') !== 'hover_intent') return;
        if (!window.__nixieWalkSupported) {
            showToast(nixieT('toast.walk_unsupported'));
            applyWalkPhaseAttr('idle');
            return;
        }
        try {
            window.ipc.postMessage('walk_start');
        } catch (e) {}
    }, WALK_HOVER_MS);
}

function walkOnPetLeave() {
    var pet = document.getElementById('pet');
    if (!pet) return;
    if (pet.getAttribute('data-walk-phase') === 'hover_intent') {
        walkClearHoverTimer();
        applyWalkPhaseAttr('idle');
    }
}

function walkOnGlobalPointerDown(e) {
    if (e.button !== 0) return;
    var pet = document.getElementById('pet');
    if (!pet) return;
    var phase = pet.getAttribute('data-walk-phase') || 'off';
    if (phase === 'off') return;
    if (phase === 'hover_intent') {
        walkClearHoverTimer();
        applyWalkPhaseAttr('idle');
        return;
    }
    if (phase === 'following') {
        try {
            window.ipc.postMessage('walk_stop');
        } catch (err) {}
    }
}

function walkOnEscape() {
    var pet = document.getElementById('pet');
    if (!pet) return;
    var phase = pet.getAttribute('data-walk-phase') || 'off';
    if (phase === 'hover_intent') {
        walkClearHoverTimer();
        applyWalkPhaseAttr('idle');
        return;
    }
    if (phase === 'following') {
        try {
            window.ipc.postMessage('walk_stop');
        } catch (err) {}
    }
}

var pomoInterval = null;
var pomoRemainingSec = 0;
/** 剩余秒数 ≤ 此值（8 分钟）时番茄气泡切为绿色 */
var POMO_GREEN_AT_SEC = 8 * 60;

function getPomoDurationSec() {
    try {
        var v = localStorage.getItem('nixie.pomodoroSec');
        if (v) return Math.max(60, parseInt(v, 10) || 1500);
    } catch (e) {}
    return 1500;
}

function isPomodoroRunning() {
    return pomoInterval != null && pomoRemainingSec > 0;
}

function formatPomoMmSs(totalSec) {
    var m = Math.floor(totalSec / 60);
    var s = totalSec % 60;
    return (m < 10 ? '0' : '') + m + ':' + (s < 10 ? '0' : '') + s;
}

function syncPomoMenuLabel() {
    var btn = document.getElementById('menu-pomo-toggle');
    if (!btn) return;
    if (isPomodoroRunning()) {
        btn.textContent = nixieT('menu.pomo_stop') + ' · ' + formatPomoMmSs(pomoRemainingSec);
    } else {
        btn.textContent = nixieT('menu.pomo_start');
    }
}

function updatePomodoroBadgeUi() {
    var pet = document.getElementById('pet');
    var pomoText = document.getElementById('pomodoro-text');
    var running = isPomodoroRunning();
    pet.setAttribute('data-pomo-running', running ? '1' : '0');
    if (running) {
        pet.setAttribute('data-pomo-phase', pomoRemainingSec <= POMO_GREEN_AT_SEC ? 'green' : 'red');
        if (pomoText) pomoText.textContent = formatPomoMmSs(pomoRemainingSec);
    } else {
        pet.removeAttribute('data-pomo-phase');
        if (pomoText) pomoText.textContent = '';
    }
    syncPomoMenuLabel();
    if (typeof NixieIdlePlay !== 'undefined') {
        if (running) NixieIdlePlay.cancel('pomo');
        else NixieIdlePlay.scheduleArm();
    }
}

function startPomodoro() {
    stopPomodoro();
    pomoRemainingSec = getPomoDurationSec();
    pomoInterval = setInterval(function() {
        pomoRemainingSec--;
        updatePomodoroBadgeUi();
        if (pomoRemainingSec <= 0) {
            stopPomodoro();
            playPomodoroBeep();
            showToast(nixieT('toast.pomo_done'));
        }
    }, 1000);
    updatePomodoroBadgeUi();
}

function stopPomodoro() {
    if (pomoInterval != null) {
        clearInterval(pomoInterval);
        pomoInterval = null;
    }
    pomoRemainingSec = 0;
    updatePomodoroBadgeUi();
}

function togglePomodoro() {
    if (isPomodoroRunning()) {
        stopPomodoro();
    } else {
        startPomodoro();
    }
}

function playPomodoroBeep() {
    if (!isNixieSoundOn()) return;
    resumeNixieAudioIfNeeded();
    playSquareSequence([784, 1046, 1318], 70, 0.07);
}

/** macOS/Windows 外圈穿透：菜单项常落在外圈透明区，须通知 Rust 暂时整窗接收点击 */
function syncPointerPassThroughForPixelMenu(open) {
    if (!window.__nixiePointerPoll || !window.ipc) return;
    window.ipc.postMessage(open ? 'menu_open' : 'menu_close');
}

/** 右键「安全退出」：告别气泡 + 沿 data-facing 方向滑出，再 IPC 关窗 */
var nixieQuitStarted = false;
var NIXIE_EXIT_ANIM_MS = 880;
/** 与 CSS `#pet.pet-enter-run` 动画时长一致 */
var NIXIE_ENTER_ANIM_MS = 950;
/** 仅进程首次展示：开场跑入未结束前，勿让 Rust 的 updateMood 清掉 class / 气泡 */
var nixieBootEntranceActive = false;
var nixieEntranceDeferred = null;

function flushNixieEntranceDeferred() {
    var d = nixieEntranceDeferred;
    nixieEntranceDeferred = null;
    if (!d) {
        if (typeof NixieIdlePlay !== 'undefined') NixieIdlePlay.onAfterMoodUpdate('idle');
        return;
    }
    if (d.kind === 'celebration') {
        updateMood(d.mood, d.label, d.hasExt, d.quote, d.focusFile, d.subtitle, true);
        applyCelebrationTier(d.tier, d.durationMs, d.isError);
    } else {
        updateMood(d.mood, d.label, d.hasExt, d.quote, d.focusFile, d.subtitle, d.skipSound);
    }
}
function beginSafeQuit() {
    if (nixieQuitStarted) return;
    nixieQuitStarted = true;
    walkClearHoverTimer();
    var pet0 = document.getElementById('pet');
    if (pet0 && pet0.getAttribute('data-walk-phase') === 'following') {
        try {
            window.ipc.postMessage('walk_stop');
        } catch (e) {}
    }
    if (typeof stopPomodoro === 'function') stopPomodoro();
    if (typeof NixieIdlePlay !== 'undefined') NixieIdlePlay.cancel('mood');
    if (celebrationClearTimer) {
        clearTimeout(celebrationClearTimer);
        celebrationClearTimer = null;
    }
    if (celebrationPauseTimer) {
        clearTimeout(celebrationPauseTimer);
        celebrationPauseTimer = null;
    }
    var pet = document.getElementById('pet');
    if (!pet) {
        if (window.ipc) window.ipc.postMessage('quit');
        return;
    }
    pet.removeAttribute('data-celebration-tier');
    pet.removeAttribute('data-celebration-ms');
    pet.removeAttribute('data-celebration-err');
    pet.removeAttribute('data-celebration-pause-motion');
    pet.removeAttribute('data-success-trivial');
    pet.classList.remove('pet-jump', 'pet-poke', 'pet-poke-comfort');
    clearPokeAsideTimer();
    clearPokeMainTimer();
    if (typeof bubbleHideTimer !== 'undefined' && bubbleHideTimer) {
        clearTimeout(bubbleHideTimer);
        bubbleHideTimer = null;
    }
    var mt = document.getElementById('mood-text');
    if (mt) mt.textContent = nixieT('quit.fleeing');
    setBubbleSubtitle('');
    setFocusFileHint('');
    var dot = document.getElementById('ext-dot');
    if (dot) dot.classList.remove('active');
    var bubble = document.getElementById('bubble');
    if (bubble) bubble.classList.add('bubble-visible');
    void pet.offsetWidth;
    pet.classList.add('pet-exit-flee');
    playPetExitCue();
    setTimeout(function() {
        if (window.ipc) window.ipc.postMessage('quit');
    }, NIXIE_EXIT_ANIM_MS + 80);
}

function closePixelMenu() {
    document.getElementById('context-menu').hidden = true;
    syncPointerPassThroughForPixelMenu(false);
}

function openAboutSheet() {
    var psModal = document.getElementById('pet-settings-modal');
    if (psModal && !psModal.hidden) closePetSettingsModal();
    var sheet = document.getElementById('about-sheet');
    if (!sheet) return;
    var meta = window.__nixieMeta || {};
    var vLine = document.getElementById('about-version-line');
    var pLine = document.getElementById('about-path-line');
    var nLine = document.getElementById('about-note-line');
    if (vLine) vLine.textContent = nixieT('about.version') + ' ' + (meta.version != null ? meta.version : '—');
    if (pLine) pLine.textContent = nixieT('about.path') + ' ' + (meta.configDir != null ? meta.configDir : '—');
    if (nLine) nLine.textContent = nixieT('about.note');
    sheet.hidden = false;
    /* 与右键菜单相同：macOS/Windows 外圈否则穿透，WebView 收不到遮罩点击 */
    syncPointerPassThroughForPixelMenu(true);
}

function closeAboutSheet() {
    var sheet = document.getElementById('about-sheet');
    if (sheet) sheet.hidden = true;
    syncPointerPassThroughForPixelMenu(false);
}

function openPixelMenu(clientX, clientY) {
    walkClearHoverTimer();
    walkOnMenuOpenResetHover();
    var m = document.getElementById('context-menu');
    refreshMenuFeedState();
    syncMenuStaticLabels();
    syncPomoMenuLabel();
    syncSoundMenuLabel();
    syncWalkMenuLabel();
    m.hidden = false;
    m.style.left = Math.min(clientX, window.innerWidth - 124) + 'px';
    m.style.top = Math.min(clientY, window.innerHeight - 280) + 'px';
    syncPointerPassThroughForPixelMenu(true);
}

function refreshMenuFeedState() {
    var pet = document.getElementById('pet');
    var can = pet.getAttribute('data-can-feed') !== '0';
    document.getElementById('menu-feed').disabled = !can;
    document.getElementById('menu-hint').textContent = can ? '' : nixieT('menu.feed_cooldown');
}

function onFeedResult(ok) {
    if (ok) {
        if (soundGateReady && isNixieSoundOn()) playFeedCue();
        showToast(nixieT('toast.feed_sweet'), true);
        setFeedAvailable(false);
        var apple = document.getElementById('feed-apple');
        apple.classList.remove('feed-apple-fly');
        void apple.offsetWidth;
        apple.classList.add('feed-apple-fly');
        setTimeout(function() {
            apple.classList.remove('feed-apple-fly');
        }, 900);
    } else {
        showToast(nixieT('toast.feed_cooldown'));
    }
    refreshMenuFeedState();
}

document.addEventListener('DOMContentLoaded', function() {
    applyPetSettingsFromRust();
    if (typeof NixieIdlePlay !== 'undefined') NixieIdlePlay.init();
    petVisualEl = document.querySelector('#pet .pet-visual');
    petLookShiftEl = document.querySelector('#pet .pet-look-shift');
    if (petVisualEl) petVisualEl.addEventListener('transitionend', onPetLookFlipTransitionEnd);
    if (petLookShiftEl) petLookShiftEl.addEventListener('transitionend', onPetLookShiftTransitionEnd);
    lookIdleFacing = document.getElementById('pet').getAttribute('data-facing') || 'right';
    lookHystSide = lookIdleFacing;
    syncLookNudgeToFacing();
    var lookField = document.getElementById('pet-look-field');
    if (lookField && SHOW_PET_LOOK_FIELD_DEBUG) lookField.classList.add('pet-look-debug');
    if (!window.__nixiePointerPoll) {
        document.addEventListener('mousemove', queuePetFacingFromPointer);
    }
    (function bootEntrance() {
        nixieBootEntranceActive = true;
        nixieEntranceDeferred = null;
        var pet = document.getElementById('pet');
        pet.className = 'mood-idle pet-enter-run';
        playPetEnterCue();
        var mt = document.getElementById('mood-text');
        if (mt) mt.textContent = nixieT('boot.here');
        setBubbleSubtitle('');
        setFocusFileHint('');
        var dot = document.getElementById('ext-dot');
        if (dot) dot.classList.remove('active');
        var bubble = document.getElementById('bubble');
        if (bubbleHideTimer) {
            clearTimeout(bubbleHideTimer);
            bubbleHideTimer = null;
        }
        if (bubble) {
            bubble.classList.add('bubble-visible');
            bubbleHideTimer = setTimeout(function() {
                if (bubble) bubble.classList.remove('bubble-visible');
                bubbleHideTimer = null;
            }, 2600);
        }
        setTimeout(function() {
            nixieBootEntranceActive = false;
            pet.classList.remove('pet-enter-run');
            flushNixieEntranceDeferred();
        }, NIXIE_ENTER_ANIM_MS);
    })();
    setTimeout(function() {
        soundGateReady = true;
    }, 1000);
    if (typeof setSoundEnabledFromRust === 'function') {
        setSoundEnabledFromRust(!!window.__nixieSoundEnabled, false);
    }

    var petEl = document.getElementById('pet');
    petEl.addEventListener('mousedown', function(e) {
        if (e.button !== 0) return;
        petPokeDragSent = false;
        petPokeDown = {
            x: e.clientX,
            y: e.clientY,
            t: Date.now(),
            onPet: petEl.contains(e.target)
        };
    });
    document.addEventListener('mousedown', walkOnGlobalPointerDown, true);
    petEl.addEventListener('mouseenter', walkOnPetEnter);
    petEl.addEventListener('mouseleave', walkOnPetLeave);
    document.addEventListener('mousemove', onPetPokePointerMove);
    document.addEventListener('mouseup', onPetPokePointerUp);
    window.addEventListener('blur', function() {
        petPokeDown = null;
        petPokeDragSent = false;
    });
    petEl.addEventListener('contextmenu', function(e) {
        e.preventDefault();
        openPixelMenu(e.clientX, e.clientY);
    });
    /* wry/WKWebView：右键菜单打开后，菜单项上 click 常不触发；改用 mousedown + 阻止冒泡 */
    function menuItemActivate(e, fn) {
        if (e.button !== 0) return;
        e.preventDefault();
        e.stopPropagation();
        closePixelMenu();
        try {
            fn();
        } catch (err) {}
    }
    document.getElementById('menu-pomo-toggle').addEventListener('mousedown', function(e) {
        menuItemActivate(e, function() {
            togglePomodoro();
        });
    });
    document.getElementById('menu-sound-toggle').addEventListener('mousedown', function(e) {
        menuItemActivate(e, function() {
            window.ipc.postMessage('sound_toggle');
        });
    });
    document.getElementById('menu-walk-toggle').addEventListener('mousedown', function(e) {
        menuItemActivate(e, function() {
            if (!window.__nixieWalkSupported) return;
            window.ipc.postMessage('walk_toggle');
        });
    });
    document.getElementById('menu-feed').addEventListener('mousedown', function(e) {
        menuItemActivate(e, function() {
            if (document.getElementById('menu-feed').disabled) return;
            window.ipc.postMessage('feed');
        });
    });
    document.getElementById('menu-open-folder').addEventListener('mousedown', function(e) {
        menuItemActivate(e, function() {
            if (window.ipc) window.ipc.postMessage('open_config_dir');
        });
    });
    document.getElementById('menu-pet-settings').addEventListener('mousedown', function(e) {
        menuItemActivate(e, function() {
            openPetSettingsModal();
        });
    });
    document.getElementById('menu-about').addEventListener('mousedown', function(e) {
        menuItemActivate(e, function() {
            openAboutSheet();
        });
    });
    document.getElementById('menu-quit').addEventListener('mousedown', function(e) {
        menuItemActivate(e, function() {
            beginSafeQuit();
        });
    });
    document.addEventListener('mousedown', function(e) {
        if (e.button !== 0) return;
        var menu = document.getElementById('context-menu');
        if (menu.hidden) return;
        if (menu.contains(e.target)) return;
        closePixelMenu();
    }, true);
    var petSettingsModal = document.getElementById('pet-settings-modal');
    if (petSettingsModal) {
        petSettingsModal.addEventListener('mousedown', function(e) {
            if (e.button !== 0) return;
            if (petSettingsModal.hidden) return;
            if (e.target === petSettingsModal) closePetSettingsModal();
        });
    }
    var petSave = document.getElementById('pet-settings-save');
    if (petSave) {
        petSave.addEventListener('mousedown', function(e) {
            if (e.button !== 0) return;
            e.preventDefault();
            e.stopPropagation();
            savePetSettingsModal();
        });
    }
    var petCancel = document.getElementById('pet-settings-cancel');
    if (petCancel) {
        petCancel.addEventListener('mousedown', function(e) {
            if (e.button !== 0) return;
            e.preventDefault();
            e.stopPropagation();
            closePetSettingsModal();
        });
    }
    var petReset = document.getElementById('pet-settings-reset');
    if (petReset) {
        petReset.addEventListener('mousedown', function(e) {
            if (e.button !== 0) return;
            e.preventDefault();
            e.stopPropagation();
            var d = defaultPetSettingsForm();
            rebuildPetSettingsSelectOptions(d.breed);
            syncPetSettingsModalLabels();
            writePetSettingsForm(d);
        });
    }
    var petHelp = document.getElementById('pet-settings-help');
    if (petHelp) {
        petHelp.addEventListener('mousedown', function(e) {
            if (e.button !== 0) return;
            e.preventDefault();
            e.stopPropagation();
            showToast(nixieT('pet.settings.help_toast'), true);
        });
    }
    var petGear = document.getElementById('pet-settings-gear');
    if (petGear) {
        petGear.addEventListener('mousedown', function(e) {
            if (e.button !== 0) return;
            e.preventDefault();
            e.stopPropagation();
            if (window.ipc) window.ipc.postMessage('open_config_dir');
        });
    }
    var aboutSheet = document.getElementById('about-sheet');
    if (aboutSheet) {
        /* 只响应点在遮罩层本体（半透明空白区），点在面板上不关 */
        aboutSheet.addEventListener('mousedown', function(e) {
            if (e.button !== 0) return;
            if (aboutSheet.hidden) return;
            if (e.target === aboutSheet) closeAboutSheet();
        });
    }
    document.addEventListener('keydown', function(e) {
        if (e.key === 'Escape') {
            var cm = document.getElementById('context-menu');
            if (cm && !cm.hidden) {
                closePixelMenu();
                return;
            }
            var ps = document.getElementById('pet-settings-modal');
            if (ps && !ps.hidden) {
                closePetSettingsModal();
                return;
            }
            var sh = document.getElementById('about-sheet');
            if (sh && !sh.hidden) {
                closeAboutSheet();
                return;
            }
            walkOnEscape();
        }
    });
    var aboutCloseBtn = document.getElementById('about-close');
    if (aboutCloseBtn) {
        aboutCloseBtn.addEventListener('mousedown', function(e) {
            if (e.button !== 0) return;
            e.preventDefault();
            closeAboutSheet();
        });
    }
});
