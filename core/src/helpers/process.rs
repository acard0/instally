use std::path::Path;

use sysinfo::{ProcessExt, System, SystemExt, Pid};

pub fn terminate_processes_under_folder<P: AsRef<Path>>(folder: P) -> Result<(), std::io::Error> {
    let current = std::process::id() as usize;
    let folder_str = folder.as_ref().to_str().unwrap();

    let sys = System::new_all();
    let processes = sys.processes();

    for process in processes {
        let parent = process.1.parent().unwrap_or(Pid::from(0));

        if process.1.exe().to_str().unwrap().contains(folder_str)
        && parent.ne(&Pid::from(current)) && process.0.ne(&Pid::from(current)) && !process.1.kill() {
            process.1.wait(); //TODO: timeout?
        }
    }

    Ok(())
}