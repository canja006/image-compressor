//! OS right-click "Compress with Image Compressor" integration (B5). Two per-OS targets:
//!
//! - **macOS**: a Finder Quick Action (Automator *Service*) installed under
//!   `~/Library/Services`, whose Run Shell Script action hands the selected files to `imgc compress`.
//! - **Windows**: `SystemFileAssociations\.<ext>\shell` registry entries (per supported extension)
//!   whose `command` runs `imgc compress … "%1"`.
//!
//! Everything that builds the integration text (the `.reg` body, the Info.plist, the workflow plist)
//! is a pure function so it can be unit-tested on any platform; only the `install_*`/`uninstall_*`
//! helpers touch the filesystem. The context menu always invokes the *same* `imgc` binary, so it
//! shares the engine code path with the GUI — nothing is re-implemented here.

use engine::SUPPORTED_EXTENSIONS;
use std::path::{Path, PathBuf};

/// Menu label shown in Finder / Explorer.
pub const MENU_LABEL: &str = "Compress with Image Compressor";
/// The macOS Quick Action bundle directory name.
pub const BUNDLE_NAME: &str = "Compress with Image Compressor.workflow";
/// Registry key leaf used for the Windows verb (stable id so uninstall can find it).
const WIN_VERB: &str = "ImageCompressorCompress";

/// The compression recipe the context menu applies, plus the resolved `imgc` path.
pub struct ShellConfig<'a> {
    pub exe: &'a Path,
    pub cap: &'a str,
    pub format: &'a str,
    pub out: Option<&'a str>,
}

impl ShellConfig<'_> {
    /// The shared middle of the command line: `compress --cap <cap> --format <format> [--out "<o>"]`.
    fn compress_args(&self) -> Vec<String> {
        let mut args = vec![
            "compress".to_string(),
            "--cap".to_string(),
            self.cap.to_string(),
            "--format".to_string(),
            self.format.to_string(),
        ];
        if let Some(out) = self.out {
            args.push("--out".to_string());
            args.push(format!("\"{out}\""));
        }
        args
    }

    /// Full Windows command line, ending in the `"%1"` per-file placeholder.
    fn windows_command(&self) -> String {
        let mut line = format!("\"{}\"", self.exe.display());
        for arg in self.compress_args() {
            line.push(' ');
            line.push_str(&arg);
        }
        line.push_str(" \"%1\"");
        line
    }

    /// Full macOS shell-script body, ending in `"$@"` (Automator passes selected files as arguments).
    pub fn macos_command(&self) -> String {
        let mut line = format!("\"{}\"", self.exe.display());
        for arg in self.compress_args() {
            line.push(' ');
            line.push_str(&arg);
        }
        line.push_str(" \"$@\"");
        line
    }
}

/// Escape a string for a `.reg` double-quoted value (backslash and quote).
fn reg_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Escape a string for XML text/attribute content in a plist.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Build the Windows `.reg` body. `install = false` produces the uninstall (`[-key]`) form.
pub fn windows_reg(cfg: &ShellConfig, install: bool) -> String {
    let mut s = String::from("Windows Registry Editor Version 5.00\r\n\r\n");
    let command = reg_escape(&cfg.windows_command());
    let icon = reg_escape(&cfg.exe.display().to_string());
    for ext in SUPPORTED_EXTENSIONS {
        let base = format!(
            "HKEY_CURRENT_USER\\Software\\Classes\\SystemFileAssociations\\.{ext}\\shell\\{WIN_VERB}"
        );
        if install {
            s.push_str(&format!("[{base}]\r\n"));
            s.push_str(&format!("@=\"{}\"\r\n", reg_escape(MENU_LABEL)));
            s.push_str(&format!("\"Icon\"=\"{icon}\"\r\n\r\n"));
            s.push_str(&format!("[{base}\\command]\r\n"));
            s.push_str(&format!("@=\"{command}\"\r\n\r\n"));
        } else {
            // Removing the verb key removes its `command` subkey too.
            s.push_str(&format!("[-{base}]\r\n\r\n"));
        }
    }
    s
}

/// The macOS Service registration plist (`Contents/Info.plist`).
pub fn macos_info_plist() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>NSServices</key>
	<array>
		<dict>
			<key>NSMenuItem</key>
			<dict>
				<key>default</key>
				<string>{label}</string>
			</dict>
			<key>NSMessage</key>
			<string>runWorkflowAsService</string>
			<key>NSRequiredContext</key>
			<dict>
				<key>NSApplicationIdentifier</key>
				<string>com.apple.finder</string>
			</dict>
			<key>NSSendFileTypes</key>
			<array>
				<string>public.image</string>
			</array>
		</dict>
	</array>
</dict>
</plist>
"#,
        label = xml_escape(MENU_LABEL)
    )
}

/// The Automator workflow document (`Contents/document.wflow`): a single Run Shell Script action that
/// receives the selected file paths as arguments and forwards them to `imgc compress`.
pub fn macos_wflow(cfg: &ShellConfig) -> String {
    let command = xml_escape(&cfg.macos_command());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>AMApplicationBuild</key>
	<string>521</string>
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
					<string>{command}</string>
					<key>CheckedForUserDefaultShell</key>
					<true/>
					<key>inputMethod</key>
					<integer>1</integer>
					<key>shell</key>
					<string>/bin/zsh</string>
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
				<string>4F1B7C2A-0001-4A10-9F00-IMAGECOMPRESS</string>
				<key>Keywords</key>
				<array>
					<string>Shell</string>
				</array>
				<key>OutputUUID</key>
				<string>4F1B7C2A-0002-4A10-9F00-IMAGECOMPRESS</string>
				<key>UUID</key>
				<string>4F1B7C2A-0003-4A10-9F00-IMAGECOMPRESS</string>
				<key>UnlocalizedApplications</key>
				<array>
					<string>Automator</string>
				</array>
				<key>arguments</key>
				<dict/>
				<key>isViewVisible</key>
				<integer>1</integer>
				<key>location</key>
				<string>309.000000:253.000000</string>
				<key>nibPath</key>
				<string>/System/Library/Automator/Run Shell Script.action/Contents/Resources/main.nib</string>
			</dict>
			<key>isViewVisible</key>
			<integer>1</integer>
		</dict>
	</array>
	<key>connectors</key>
	<dict/>
	<key>workflowMetaData</key>
	<dict>
		<key>applicationBundleIDsByPath</key>
		<dict/>
		<key>applicationPaths</key>
		<array/>
		<key>inputTypeIdentifier</key>
		<string>com.apple.Automator.fileSystemObject.image</string>
		<key>outputTypeIdentifier</key>
		<string>com.apple.Automator.nothing</string>
		<key>presentationMode</key>
		<integer>11</integer>
		<key>processesInput</key>
		<integer>0</integer>
		<key>serviceApplicationBundleID</key>
		<string>com.apple.finder</string>
		<key>serviceApplicationPath</key>
		<string>/System/Library/CoreServices/Finder.app</string>
		<key>serviceInputTypeIdentifier</key>
		<string>com.apple.Automator.fileSystemObject.image</string>
		<key>serviceOutputTypeIdentifier</key>
		<string>com.apple.Automator.nothing</string>
		<key>serviceProcessesInput</key>
		<integer>0</integer>
		<key>systemImageName</key>
		<string>NSActionTemplate</string>
		<key>useAutomaticInputType</key>
		<integer>0</integer>
		<key>workflowTypeIdentifier</key>
		<string>com.apple.Automator.servicesMenu</string>
	</dict>
</dict>
</plist>
"#,
        command = command
    )
}

/// Install the macOS Quick Action into `services_dir` (normally `~/Library/Services`). Returns the
/// created bundle path.
pub fn install_macos(services_dir: &Path, cfg: &ShellConfig) -> std::io::Result<PathBuf> {
    let bundle = services_dir.join(BUNDLE_NAME);
    let contents = bundle.join("Contents");
    std::fs::create_dir_all(&contents)?;
    std::fs::write(contents.join("Info.plist"), macos_info_plist())?;
    std::fs::write(contents.join("document.wflow"), macos_wflow(cfg))?;
    Ok(bundle)
}

/// Remove the macOS Quick Action bundle from `services_dir`. Returns whether it existed.
pub fn uninstall_macos(services_dir: &Path) -> std::io::Result<bool> {
    let bundle = services_dir.join(BUNDLE_NAME);
    if bundle.exists() {
        std::fs::remove_dir_all(&bundle)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> ShellConfig<'static> {
        ShellConfig {
            exe: Path::new("/Applications/imgc"),
            cap: "500k",
            format: "keep",
            out: None,
        }
    }

    #[test]
    fn windows_reg_registers_every_supported_extension() {
        let reg = windows_reg(&cfg(), true);
        assert!(reg.starts_with("Windows Registry Editor Version 5.00"));
        for ext in SUPPORTED_EXTENSIONS {
            assert!(
                reg.contains(&format!(
                    "SystemFileAssociations\\.{ext}\\shell\\{WIN_VERB}"
                )),
                "missing the .{ext} verb key"
            );
        }
        // The command runs `imgc compress … "%1"` with the menu label present.
        assert!(reg.contains("compress --cap 500k --format keep"));
        assert!(
            reg.contains("\\\"%1\\\""),
            "quotes around %1 must be escaped"
        );
        assert!(reg.contains(MENU_LABEL));
    }

    #[test]
    fn windows_uninstall_deletes_the_keys() {
        let reg = windows_reg(&cfg(), false);
        for ext in SUPPORTED_EXTENSIONS {
            assert!(reg.contains(&format!(
                "[-HKEY_CURRENT_USER\\Software\\Classes\\SystemFileAssociations\\.{ext}\\shell\\{WIN_VERB}]"
            )));
        }
        assert!(
            !reg.contains("compress --cap"),
            "uninstall embeds no command"
        );
    }

    #[test]
    fn windows_reg_escapes_a_spaced_exe_path() {
        let exe = PathBuf::from("C:\\Program Files\\Image Compressor\\imgc.exe");
        let c = ShellConfig {
            exe: &exe,
            cap: "2mb",
            format: "jpeg",
            out: None,
        };
        let reg = windows_reg(&c, true);
        // Backslashes are doubled and the quoted exe path survives escaping.
        assert!(reg.contains("C:\\\\Program Files\\\\Image Compressor\\\\imgc.exe"));
    }

    #[test]
    fn macos_command_forwards_selected_files() {
        let out = "/tmp/out";
        let c = ShellConfig {
            exe: Path::new("/Applications/imgc"),
            cap: "1mb",
            format: "jpeg",
            out: Some(out),
        };
        let cmd = c.macos_command();
        assert_eq!(
            cmd,
            "\"/Applications/imgc\" compress --cap 1mb --format jpeg --out \"/tmp/out\" \"$@\""
        );
    }

    #[test]
    fn macos_wflow_embeds_the_xml_escaped_command() {
        let wflow = macos_wflow(&cfg());
        // The quotes in the command are XML-escaped inside the plist string.
        assert!(wflow.contains("&quot;/Applications/imgc&quot; compress --cap 500k"));
        assert!(wflow.contains("com.apple.Automator.servicesMenu"));
        assert!(wflow.contains("/System/Library/Automator/Run Shell Script.action"));
    }

    #[test]
    fn install_and_uninstall_round_trip() {
        let dir = std::env::temp_dir().join(format!("imgc_shell_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp services dir");

        let bundle = install_macos(&dir, &cfg()).expect("install");
        assert!(bundle.join("Contents/Info.plist").exists());
        assert!(bundle.join("Contents/document.wflow").exists());

        assert!(uninstall_macos(&dir).expect("uninstall"), "bundle existed");
        assert!(!bundle.exists(), "bundle removed");
        assert!(
            !uninstall_macos(&dir).expect("second uninstall"),
            "second uninstall is a no-op"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The generated plists must be well-formed (verified with the system `plutil`).
    #[test]
    #[cfg(target_os = "macos")]
    fn generated_plists_are_well_formed() {
        let dir = std::env::temp_dir().join(format!("imgc_plutil_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let info = dir.join("Info.plist");
        let wflow = dir.join("document.wflow");
        std::fs::write(&info, macos_info_plist()).expect("write info");
        std::fs::write(&wflow, macos_wflow(&cfg())).expect("write wflow");

        for file in [&info, &wflow] {
            let status = std::process::Command::new("plutil")
                .arg("-lint")
                .arg(file)
                .status()
                .expect("run plutil");
            assert!(
                status.success(),
                "plutil -lint failed for {}",
                file.display()
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
