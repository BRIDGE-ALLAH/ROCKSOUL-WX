#define AppName "ROCKSOUL-WX"
#define AppPublisher "BRIDGE-ALLAH"
#ifndef AppVersion
  #error "AppVersion must be supplied, for example /DAppVersion=0.2.0"
#endif

[Setup]
AppId={{6A5EAB9C-594D-4E2B-9B7B-9462B30D62B1}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={localappdata}\Programs\ROCKSOUL-WX
DefaultGroupName=ROCKSOUL-WX
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
OutputDir=..\..\dist\windows
OutputBaseFilename=ROCKSOUL-WX-Setup-{#AppVersion}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\wx-gateway.exe
Uninstallable=yes

[Files]
Source: "..\..\target\release\wx-gateway.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "Start-Gateway.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "Test-Gateway.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "WINDOWS-README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Start Local Gateway"; Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoExit -ExecutionPolicy Bypass -File ""{app}\Start-Gateway.ps1"""; WorkingDir: "{app}"
Name: "{group}\Verify Local Gateway"; Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoExit -ExecutionPolicy Bypass -File ""{app}\Test-Gateway.ps1"""; WorkingDir: "{app}"
Name: "{group}\Windows Testing Guide"; Filename: "{app}\WINDOWS-README.md"
Name: "{group}\Uninstall ROCKSOUL-WX"; Filename: "{uninstallexe}"
