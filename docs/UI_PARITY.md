# WinCraft — L3 Desktop UI Parity (`wincraft-app`)

Every page in `wincraft-app` calls `wincraft_core::dispatch` in-process (verified by `app_dispatch_matches_core_dispatch`):

| L3 Page | Core `Command` Dispatched |
|---|---|
| **1. Host & Disks** | `Command::Status`, `Command::DiskList`, `Command::EnvAudit` |
| **2. Processes & Ports** | `Command::ProcList`, `Command::PortMap` |
| **3. RAM Weeder** | `Command::WeederScan`, `Command::WeederReap` |
| **4. High-Speed FileOps** | `Command::FsDu`, `Command::FsDups`, `Command::FsLarge`, `Command::FsGrep` |
| **5. Who Locks This File?** | `Command::LockInspect` (Win32 Restart Manager `rstrtmgr.dll`) |
| **6. Services & NSSM** | `Command::SvcList`, `Command::SvcControl` |
| **7. Startup & Registry** | `Command::StartupList`, `Command::RegRead`, `Command::EventTail` |
| **8. Doctor & Tools (26)** | `Command::Doctor`, `Command::Tools` |
