/**
 * 最小 i18n：无依赖、全内联；Rust 可在注入 HTML 前设 window.__nixieLocale = 'en'|'ja'|'zh'。
 * 缺失翻译回退到 zh，再回退到 key（便于发现漏键）。
 */
(function() {
    var FALLBACK_LOCALE = 'zh';

    var NIXIE_I18N = {
        zh: {
            'menu.sound_on': '小猪音效 · 开',
            'menu.sound_off': '小猪音效 · 关',
            'menu.walk_on': '溜猪 · 开',
            'menu.walk_off': '溜猪 · 关',
            'menu.walk_unsupported': '溜猪 · 需桌面版',
            'menu.pomo_start': '番茄钟 · 开始',
            'menu.pomo_stop': '番茄钟 · 停止',
            'menu.feed': '投喂小苹果',
            'menu.open_folder': '打开配置目录',
            'menu.about': '关于小猪…',
            'menu.quit': '安全退出小猪',
            'menu.feed_cooldown': '投喂冷却中…',
            'about.title': 'Codepet 小猪',
            'about.version': '版本',
            'about.path': '配置目录',
            'about.close': '好的',
            'about.note': '台词、音效与窗口位置等都在配置目录。未来将仅以菜单栏收纳，不在程序坞显示图标。',
            'toast.pomo_done': '专注时间到！',
            'toast.feed_sweet': '好甜！',
            'toast.feed_cooldown': '还在冷却中…',
            'toast.walk_unsupported': '溜猪需要 macOS / Windows 窗口跟随。',
            'toast.exec_success': '执行成功！',
            'toast.user_typing': '哒哒哒',
            'quit.fleeing': '猪先溜了',
            'boot.here': '猪来也！',
            'poke.bubble': [
                '嘻嘻，有点痒。',
                '嘿嘿～再一下下～',
                '别闹……好吧再摸一下。',
                '蹭蹭～今天手感不错～'
            ],
            'poke.wake': [
                '哎呀，我刚刚没睡着。',
                '哎呀，正梦到精彩的时候！',
                '等等……梦都要被你戳成连续剧了～',
                '口水还没擦完就被发现了……'
            ],
            'walk.start': [
                '出发！今天遛哪条街？',
                '跟上我，别走丢～',
                '绳……啊不，默契牵好了，走！',
                '小碎步踩起来～'
            ],
            'walk.end': [
                '好啦，歇会儿，脚酸了。',
                '今天就遛到这儿～',
                '停！再走下去要开导航了。',
                '收队，回家喝水～'
            ]
        },
        en: {
            'menu.sound_on': 'Pet sounds · On',
            'menu.sound_off': 'Pet sounds · Off',
            'menu.walk_on': 'Walk the pet · On',
            'menu.walk_off': 'Walk the pet · Off',
            'menu.walk_unsupported': 'Walk · Desktop only',
            'menu.pomo_start': 'Pomodoro · Start',
            'menu.pomo_stop': 'Pomodoro · Stop',
            'menu.feed': 'Feed a tiny apple',
            'menu.open_folder': 'Open config folder',
            'menu.about': 'About…',
            'menu.quit': 'Quit safely',
            'menu.feed_cooldown': 'Feeding on cooldown…',
            'about.title': 'Codepet',
            'about.version': 'Version',
            'about.path': 'Config folder',
            'about.close': 'OK',
            'about.note': 'Quotes, sounds, and window position live in the config folder. A future build will live in the menu bar only (no Dock icon).',
            'toast.pomo_done': 'Focus time is up!',
            'toast.feed_sweet': 'So sweet!',
            'toast.feed_cooldown': 'Still on cooldown…',
            'toast.walk_unsupported': 'Walking needs macOS / Windows.',
            'toast.exec_success': 'Done!',
            'toast.user_typing': 'Tap tap tap~',
            'quit.fleeing': 'Gotta scoot!',
            'boot.here': 'Here I come!',
            'poke.bubble': [
                'Hehe, that tickles.',
                'Okay okay—one more poke~',
                'Hey… fine, one more.',
                'Nuzzle nuzzle—good vibes today~'
            ],
            'poke.wake': [
                'I wasn’t asleep, promise.',
                'You woke me at the good part!',
                'Hey—you’re turning my dream into a series~',
                'Caught before I could wipe the drool…'
            ],
            'walk.start': [
                'Off we go—where to today?',
                'Stick with me—don’t get lost!',
                'Leash… uh, I mean vibes—let’s roll!',
                'Tiny steps, let’s go~'
            ],
            'walk.end': [
                'Okay, break time—my feet are tired.',
                'That’s enough walk for now~',
                'Stop—or we’ll need a map!',
                'Heading home for water~'
            ]
        },
        ja: {
            'menu.sound_on': '効果音 · オン',
            'menu.sound_off': '効果音 · オフ',
            'menu.walk_on': 'お散歩 · オン',
            'menu.walk_off': 'お散歩 · オフ',
            'menu.walk_unsupported': 'お散歩 · 要デスクトップ',
            'menu.pomo_start': 'ポモドーロ · 開始',
            'menu.pomo_stop': 'ポモドーロ · 停止',
            'menu.feed': 'ミニりんごをあげる',
            'menu.open_folder': '設定フォルダを開く',
            'menu.about': 'このアプリについて…',
            'menu.quit': '安全に終了',
            'menu.feed_cooldown': 'あげられるまで待ってね…',
            'about.title': 'Codepet',
            'about.version': 'バージョン',
            'about.path': '設定フォルダ',
            'about.close': 'OK',
            'about.note': 'セリフや効果音、ウィンドウ位置は設定フォルダに保存されます。将来はメニューバー専用（Dock に出さない）予定です。',
            'toast.pomo_done': '集中タイム終了！',
            'toast.feed_sweet': 'あま〜い！',
            'toast.feed_cooldown': 'まだ冷却中だよ…',
            'toast.walk_unsupported': 'お散歩は macOS / Windows で。',
            'toast.exec_success': 'できた！',
            'toast.user_typing': 'タッタッタ〜',
            'quit.fleeing': 'ばいばい〜',
            'boot.here': 'ただいま〜！',
            'poke.bubble': [
                'くすぐったい〜',
                'も、もう一回だけね',
                'もう…仕方ない、一回だけ',
                'すりすり〜今日はいい感じ'
            ],
            'poke.wake': [
                'ね、寝てなかったよ？',
                'いいところだったのに〜',
                '待って、夢が続き物になっちゃう〜',
                'よだれふく前にバレた…'
            ],
            'walk.start': [
                'いよ～っ、今日はどっち行く？',
                'はぐれないでね～',
                'リード…じゃなくてテンション、いくよ！',
                'ちょこちょこ歩こ～'
            ],
            'walk.end': [
                'はい、休憩。足パンパン。',
                '今日のお散歩はここまで～',
                'ストップ！これ以上は地図が必要…',
                '帰ってお水飲も～'
            ]
        }
    };

    function nixieResolveLocale() {
        var L = window.__nixieLocale;
        if (L && NIXIE_I18N[L]) return L;
        return FALLBACK_LOCALE;
    }

    function nixieT(key) {
        var L = nixieResolveLocale();
        var pack = NIXIE_I18N[L];
        var v = pack && pack[key];
        if (typeof v === 'string' && v.length) return v;
        var fb = NIXIE_I18N[FALLBACK_LOCALE][key];
        if (typeof fb === 'string') return fb;
        return key;
    }

    function nixieLines(key) {
        var L = nixieResolveLocale();
        function linesFrom(pack) {
            var v = pack && pack[key];
            return Array.isArray(v) ? v : null;
        }
        var arr = linesFrom(NIXIE_I18N[L]) || linesFrom(NIXIE_I18N[FALLBACK_LOCALE]);
        return arr ? arr.slice() : [];
    }

    function nixiePickLine(key) {
        var lines = nixieLines(key);
        if (!lines.length) return '';
        return lines[Math.floor(Math.random() * lines.length)];
    }

    window.nixieT = nixieT;
    window.nixieLines = nixieLines;
    window.nixiePickLine = nixiePickLine;
})();
