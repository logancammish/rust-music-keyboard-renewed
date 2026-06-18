[Setup]
AppName=Rust Music Keyboard Renewed
AppVersion=0.4.0
AppPublisher=Logan Cammish
DefaultDirName={pf}\RustMusicKeyboardRenewed
DefaultGroupName=Rust Music Keyboard Renewed
OutputDir=.
OutputBaseFilename=RustMusicKeyboardRenewed_Installer-Windows-x86_64
Compression=lzma
SolidCompression=yes

; Paths are relative to this script, so the installer builds on any machine.
[Files]
Source: "target\release\RustMusicKeyboardRenewed.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "assets\*"; DestDir: "{app}\assets"; Flags: recursesubdirs createallsubdirs
Source: "config\*"; DestDir: "{app}\config"; Flags: recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Rust Music Keyboard Renewed"; Filename: "{app}\RustMusicKeyboardRenewed.exe"
Name: "{commondesktop}\Rust Music Keyboard Renewed"; Filename: "{app}\RustMusicKeyboardRenewed.exe"

[Run]
Filename: "{app}\RustMusicKeyboardRenewed.exe"; Description: "Launch Rust Music Keyboard Renewed"; Flags: nowait postinstall skipifsilent
