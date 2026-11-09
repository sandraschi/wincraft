# WinCraft — L3 Desktop UI Parity (`wincraft-app`)

Every page in `wincraft-app` calls `wincraft_core::dispatch` in-process (verified by `app_dispatch_matches_core_dispatch_and_defaults_to_cards`) and renders through the shared `render_dataset_controls_and_body` component:

- **Default View Mode:** `OutputFormat::Cards` (`🗂 Cards (Default)`), switchable to `☰ List / Table` or `{ } JSON`.
- **Pagination:** `Page size` selector (`10`, `25`, `50`, `100`) + `<< First`, `< Prev`, `Page X / Y`, `Next >`, `Last >>`.
- **Sorting & Filtering:** Live substring `Filter:` input + clickable column sort pills/headers (`▲` / `▼`, numeric-aware).
- **Export:** One-click `CSV`, `MD`, `TXT`, and `JSON` export buttons writing to `D:\Dev\repos\wincraft\exports\<dataset>_<timestamp>.<ext>`.

| L3 Page | Core `Command` Dispatched | Dataset Slug(s) |
|---|---|---|
| **1. Host & Disks** | `Command::Status`, `Command::DiskList`, `Command::EnvAudit` | `disks`, `status` |
| **2. Processes & Ports** | `Command::ProcList`, `Command::PortMap` | `ports`, `processes` |
| **3. RAM Weeder** | `Command::WeederScan`, `Command::WeederReap` | `weeder` |
| **4. High-Speed FileOps** | `Command::FsList`, `Command::FsDu`, `Command::FsDups`, `Command::FsLarge`, `Command::FsGrep` | `fileops` |
| **5. Who Locks This File?** | `Command::LockInspect` (Win32 Restart Manager `rstrtmgr.dll`) | `locks` |
| **6. Services & NSSM** | `Command::SvcList`, `Command::SvcControl` | `services` |
| **7. Startup & Registry** | `Command::StartupList`, `Command::RegRead`, `Command::EventTail` | `registry`, `startup`, `events` |
| **8. Doctor & Tools (26)** | `Command::Doctor`, `Command::Tools` | `tools` |
