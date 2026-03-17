use sysinfo::System;

pub struct CursorProcess {
    pub running: bool,
    pub cpu_percent: f32,
}

/// Scans running processes for Cursor and samples CPU usage.
pub fn probe_cursor(sys: &mut System) -> CursorProcess {
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut running = false;
    let mut total_cpu: f32 = 0.0;

    for (_pid, proc) in sys.processes() {
        let name = proc.name().to_string_lossy();
        // Cursor's main process on macOS is "Cursor Helper (Renderer)" or "Cursor"
        if name.contains("Cursor") {
            running = true;
            total_cpu += proc.cpu_usage();
        }
    }

    CursorProcess {
        running,
        cpu_percent: total_cpu,
    }
}
