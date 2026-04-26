use crate::path_utils::get_jobs_dir;
use std::fs;

pub fn setup_os_integration() -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        setup_macos_quick_action()?;
    }
    #[cfg(target_os = "windows")]
    {
        setup_windows_send_to()?;
    }
    #[cfg(target_os = "linux")]
    {
        setup_linux_nautilus_script()?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn setup_macos_quick_action() -> anyhow::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?;
    let services_dir = home.join("Library/Services");
    fs::create_dir_all(&services_dir)?;

    let workflow_dir = services_dir.join("Backup to Shadow.workflow");
    let contents_dir = workflow_dir.join("Contents");
    fs::create_dir_all(&contents_dir)?;

    let jobs_dir = get_jobs_dir();
    let jobs_dir_str = jobs_dir.to_string_lossy();

    // A minimal macOS Services workflow plist that runs a shell script
    let info_plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>NSServices</key>
    <array>
        <dict>
            <key>NSMenuItem</key>
            <dict>
                <key>default</key>
                <string>Backup to Shadow</string>
            </dict>
            <key>NSMessage</key>
            <string>runWorkflowAsService</string>
            <key>NSRequiredContext</key>
            <dict/>
            <key>NSSendFileTypes</key>
            <array>
                <string>public.item</string>
                <string>public.folder</string>
                <string>public.directory</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
"#.to_string();

    fs::write(contents_dir.join("Info.plist"), info_plist)?;

    // The actual shell script that creates the .shadow_job file
    let document_wflow = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>AMApplicationBuild</key>
    <string>521.1</string>
    <key>AMApplicationVersion</key>
    <string>2.10</string>
    <key>AMDocumentVersion</key>
    <string>2</string>
    <key>actions</key>
    <array>
        <dict>
            <key>action</key>
            <dict>
                <key>AMAccepts</key>
                <dict>
                    <key>Container</key>
                    <string>List</string>
                    <key>Optional</key>
                    <true/>
                    <key>Types</key>
                    <array>
                        <string>com.apple.cocoa.string</string>
                    </array>
                </dict>
                <key>AMActionVersion</key>
                <string>2.0.3</string>
                <key>AMApplication</key>
                <array>
                    <string>Automator</string>
                </array>
                <key>AMParameterProperties</key>
                <dict>
                    <key>COMMAND_STRING</key>
                    <dict/>
                    <key>CheckedForUserDefaultShell</key>
                    <dict/>
                    <key>inputMethod</key>
                    <dict/>
                    <key>shell</key>
                    <dict/>
                    <key>source</key>
                    <dict/>
                </dict>
                <key>AMProvides</key>
                <dict>
                    <key>Container</key>
                    <string>List</string>
                    <key>Types</key>
                    <array>
                        <string>com.apple.cocoa.string</string>
                    </array>
                </dict>
                <key>ActionBundlePath</key>
                <string>/System/Library/Automator/Run Shell Script.action</string>
                <key>ActionName</key>
                <string>Run Shell Script</string>
                <key>ActionParameters</key>
                <dict>
                    <key>COMMAND_STRING</key>
                    <string>for f in "$@"
do
    echo "$f" > "{jobs_dir_str}/$(uuidgen).shadow_job"
done</string>
                    <key>CheckedForUserDefaultShell</key>
                    <true/>
                    <key>inputMethod</key>
                    <integer>1</integer>
                    <key>shell</key>
                    <string>/bin/bash</string>
                    <key>source</key>
                    <string></string>
                </dict>
                <key>BundleIdentifier</key>
                <string>com.apple.Automator.RunShellScript</string>
                <key>CFBundleVersion</key>
                <string>2.0.3</string>
                <key>CanShowSelectedItemsWhenRun</key>
                <false/>
                <key>CanShowWhenRun</key>
                <true/>
                <key>Category</key>
                <array>
                    <string>AMCategoryUtilities</string>
                </array>
                <key>Class Name</key>
                <string>RunShellScriptAction</string>
                <key>InputUUID</key>
                <string>D8A58525-4B80-4D54-8B41-7A39828469C5</string>
                <key>Keywords</key>
                <array>
                    <string>Shell</string>
                    <string>Script</string>
                    <string>Command</string>
                    <string>Run</string>
                    <string>Unix</string>
                </array>
                <key>OutputUUID</key>
                <string>F9D1E8F2-A685-4D55-896B-0667C29377B8</string>
                <key>UUID</key>
                <string>2E80E53B-2E85-4A3C-9A5C-7E43C88617D5</string>
                <key>UnlocalizedApplications</key>
                <array>
                    <string>Automator</string>
                </array>
                <key>arguments</key>
                <dict>
                    <key>0</key>
                    <dict>
                        <key>default value</key>
                        <integer>0</integer>
                        <key>name</key>
                        <string>inputMethod</string>
                        <key>required</key>
                        <string>0</string>
                        <key>type</key>
                        <string>0</string>
                        <key>uuid</key>
                        <string>0</string>
                    </dict>
                    <key>1</key>
                    <dict>
                        <key>default value</key>
                        <false/>
                        <key>name</key>
                        <string>CheckedForUserDefaultShell</string>
                        <key>required</key>
                        <string>0</string>
                        <key>type</key>
                        <string>0</string>
                        <key>uuid</key>
                        <string>1</string>
                    </dict>
                    <key>2</key>
                    <dict>
                        <key>default value</key>
                        <string></string>
                        <key>name</key>
                        <string>source</string>
                        <key>required</key>
                        <string>0</string>
                        <key>type</key>
                        <string>0</string>
                        <key>uuid</key>
                        <string>2</string>
                    </dict>
                    <key>3</key>
                    <dict>
                        <key>default value</key>
                        <string></string>
                        <key>name</key>
                        <string>COMMAND_STRING</string>
                        <key>required</key>
                        <string>0</string>
                        <key>type</key>
                        <string>0</string>
                        <key>uuid</key>
                        <string>3</string>
                    </dict>
                    <key>4</key>
                    <dict>
                        <key>default value</key>
                        <string>/bin/sh</string>
                        <key>name</key>
                        <string>shell</string>
                        <key>required</key>
                        <string>0</string>
                        <key>type</key>
                        <string>0</string>
                        <key>uuid</key>
                        <string>4</string>
                    </dict>
                </dict>
                <key>isViewVisible</key>
                <integer>1</integer>
                <key>location</key>
                <string>309.000000:305.000000</string>
                <key>nibPath</key>
                <string>/System/Library/Automator/Run Shell Script.action/Contents/Resources/Base.lproj/main.nib</string>
            </dict>
            <key>isViewVisible</key>
            <integer>1</integer>
        </dict>
    </array>
    <key>connectors</key>
    <dict/>
    <key>workflowMetaData</key>
    <dict>
        <key>serviceInputTypeIdentifier</key>
        <string>com.apple.Automator.fileSystemObject</string>
        <key>serviceOutputTypeIdentifier</key>
        <string>com.apple.Automator.nothing</string>
        <key>serviceProcessesInputAsData</key>
        <integer>0</integer>
        <key>workflowTypeIdentifier</key>
        <string>com.apple.Automator.servicesMenu</string>
    </dict>
</dict>
</plist>
"#
    );

    fs::write(contents_dir.join("document.wflow"), document_wflow)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn setup_windows_send_to() -> anyhow::Result<()> {
    let app_data = std::env::var("APPDATA").map(PathBuf::from)?;
    let send_to_dir = app_data.join("Microsoft/Windows/SendTo");
    fs::create_dir_all(&send_to_dir)?;

    let jobs_dir = get_jobs_dir();
    let jobs_dir_str = jobs_dir.to_string_lossy();

    // On Windows, the simplest way is a .bat file that generates a .shadow_job file.
    // We'll name it "Shadow.bat" so it appears as "Shadow" in the menu.
    let bat_content = format!(
        r#"@echo off
setlocal enabledelayedexpansion
set "job_id=%RANDOM%%RANDOM%"
echo %~1 > "{jobs_dir_str}\!job_id!.shadow_job"
"#
    );

    fs::write(send_to_dir.join("Shadow.bat"), bat_content)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn setup_linux_nautilus_script() -> anyhow::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?;
    let scripts_dir = home.join(".local/share/nautilus/scripts");
    fs::create_dir_all(&scripts_dir)?;

    let jobs_dir = get_jobs_dir();
    let jobs_dir_str = jobs_dir.to_string_lossy();

    let script_content = format!(
        r#"#!/bin/bash
for f in "$@"
do
    echo "$(realpath "$f")" > "{jobs_dir_str}/$(uuidgen).shadow_job"
done
"#
    );

    let script_path = scripts_dir.join("Backup to Shadow");
    fs::write(&script_path, script_content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms)?;
    }

    Ok(())
}
