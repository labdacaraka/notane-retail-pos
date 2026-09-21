; =====================================================================
; Inno Setup Script: Notane Retail POS Enterprise (One-Click Setup)
; Memaketkan Notane POS + Silent PostgreSQL Installer (Opsi A)
; Pengguna awam cukup klik 'Next > Install > Finish' tanpa setup database manual.
; =====================================================================

#define MyAppName "Notane Retail POS Enterprise"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "Labda Caraka / Notane Retail"
#define MyAppURL "https://github.com/labdacaraka/notane-retail-pos"
#define MyAppExeName "notane-retail-pos.exe"

[Setup]
AppId={{9F573C6E-4E7B-4B3A-8F51-893C6FE7A001}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\Notane Retail POS
DisableProgramGroupPage=yes
OutputBaseFilename=Notane-Retail-POS-Installer-x64
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; 1. Binary Utama Aplikasi (Hasil build Tauri)
Source: "..\..\src-tauri\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

; 2. Silent PostgreSQL Installer (Otomatis diekstrak ke temporary folder saat setup)
; Letakkan file postgresql-16.x-windows-x64.exe di folder installer\windows\prerequisites\
Source: "prerequisites\postgresql-installer.exe"; DestDir: "{tmp}"; Flags: deleteafterinstall ignoreversion; Check: NeedsPostgresInstall

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
; 1. Eksekusi Instalasi PostgreSQL secara diam-diam (Silent / Unattended)
; Superuser 'postgres', Password 'postgres', Port 5432
Filename: "{tmp}\postgresql-installer.exe"; \
    Parameters: "--mode unattended --superpassword postgres --serverport 5432 --enable_acdr 0"; \
    StatusMsg: "Memasang & mengonfigurasi Database Engine Notane (di balik layar)..."; \
    Flags: waituntilterminated runhidden; \
    Check: NeedsPostgresInstall

; 2. Buat Database notane_pos secara otomatis
Filename: "{cmd}"; \
    Parameters: "/c ""C:\Program Files\PostgreSQL\16\bin\psql.exe"" -U postgres -h 127.0.0.1 -p 5432 -c ""CREATE DATABASE notane_pos;"""; \
    Environment: "PGPASSWORD=postgres"; \
    Flags: runhidden waituntilterminated; \
    Check: NeedsPostgresInstall

; 3. Opsi Jalankan Notane POS setelah instalasi selesai
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
// Fungsi pengecekan: Apakah PostgreSQL sudah terpasang dan service-nya aktif?
function NeedsPostgresInstall(): Boolean;
var
  ErrorCode: Integer;
begin
  // Jika direktori instalasi PostgreSQL 16 belum ada, instal PostgreSQL
  if not DirExists('C:\Program Files\PostgreSQL\16') then
  begin
    Result := True;
  end
  else
  begin
    // PostgreSQL sudah ada di sistem, lewati instalasi ulang
    Result := False;
  end;
end;
