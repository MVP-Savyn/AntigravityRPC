#define MyAppName "Antigravity Rich Presence"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "Antigravity Community"
#define MyAppExeName "AntigravityRichPresence.exe"

[Setup]
AppId={{8D6B4A2E-5C31-4B6B-9B7A-14A3E9F4C712}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={localappdata}\Programs\Antigravity Rich Presence
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=installer_out
OutputBaseFilename=AntigravityRichPresenceSetup
SetupIconFile=assets\antigravity-icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=force

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
spanish.StartupTask=Iniciar automáticamente con Windows
english.StartupTask=Start automatically with Windows
spanish.LauncherTask=Crear acceso directo "Antigravity (con Discord RPC)" en Escritorio (se abre y cierra con Antigravity)
english.LauncherTask=Create "Antigravity (with Discord RPC)" Desktop shortcut (starts and closes with Antigravity)
spanish.DesktopTask=Crear acceso directo en el Escritorio (solo RPC)
english.DesktopTask=Create a Desktop shortcut (RPC only)
spanish.ConfigureShortcut=Configurar Antigravity Rich Presence
english.ConfigureShortcut=Configure Antigravity Rich Presence
spanish.LauncherShortcutName=Antigravity (con Discord RPC)
english.LauncherShortcutName=Antigravity (with Discord RPC)
spanish.LaunchAfterInstall=Iniciar {#MyAppName} ahora
english.LaunchAfterInstall=Launch {#MyAppName} now

[Tasks]
Name: "launchershortcut"; Description: "{cm:LauncherTask}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "startup"; Description: "{cm:StartupTask}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "desktopicon"; Description: "{cm:DesktopTask}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "dist\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "config.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{cm:LauncherShortcutName}"; Filename: "{app}\{#MyAppExeName}"; Parameters: "--launch"; IconFilename: "{app}\assets\antigravity-icon.ico"
Name: "{autodesktop}\{cm:LauncherShortcutName}"; Filename: "{app}\{#MyAppExeName}"; Parameters: "--launch"; Tasks: launchershortcut; IconFilename: "{app}\assets\antigravity-icon.ico"
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\assets\antigravity-icon.ico"
Name: "{autoprograms}\{cm:ConfigureShortcut}"; Filename: "{app}\{#MyAppExeName}"; Parameters: "--setup"; IconFilename: "{app}\assets\antigravity-icon.ico"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon; IconFilename: "{app}\assets\antigravity-icon.ico"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "AntigravityDiscordRPC"; ValueData: """{app}\{#MyAppExeName}"""; Tasks: startup; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchAfterInstall}"; Flags: nowait postinstall skipifsilent

[Code]
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#MyAppExeName}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Sleep(250);
  Result := '';
end;
