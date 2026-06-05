```text
         _____                               
   _____/ ___/____ __   _____  _____         
  / ___/\__ \/ __ `/ | / / _ \/ ___/         
 / /   ___/ / /_/ /| |/ /  __/ /             
/_/   /____/\__,_/ |___/\___/_/              
```

If it detects missing directories, incorrect screensaver files, or out-of-sync registry settings, you can instruct rSaver to heal itself automatically:

```powershell
rsav doctor --fix
```

---

## 📄 Step 2: Check the Logs

rSaver logs all events, system metrics, and download status to a background log file. This file contains valuable context if the application crashed or if a download failed.

* **Log Location**: `%APPDATA%\rSaver\rSaver.log`
* **How to open (PowerShell)**:
  ```powershell
  notepad "$env:APPDATA\rSaver\rSaver.log"
  ```

---

## 💬 Step 3: Open an Issue

If the doctor tool did not resolve your issue and you found an error in the logs, please open an issue in the official repository:

* **File a Bug or Feature Request**: [Open a GitHub Issue](https://github.com/tourian-dynamics/rSaver/issues)
* **What to include**:
  * Your Windows version (e.g., Windows 11 23H2).
  * The terminal environment you are using (e.g., PowerShell 7, Command Prompt, Windows Terminal).
  * The relevant output or error logs from `%APPDATA%\rSaver\rSaver.log`.
  * Steps to reproduce the bug.
