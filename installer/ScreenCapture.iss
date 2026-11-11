; Installeur ScreenCapture (Inno Setup 6).
; Compilé par build-installer.ps1, qui fournit AppVersion depuis Cargo.toml.

#ifndef AppVersion
  #define AppVersion "0.1.11"
#endif
#define AppName "ScreenCapture"
#define AppExe "ScreenCapture.exe"
#define FfmpegUrl "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"

[Setup]
AppId={{6F1C2D7E-3B8A-4E59-9A41-5C2E8D0B7F13}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=Stéphane Salou
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
; Installation pour l'utilisateur courant par défaut ; « pour tous » possible (droits administrateur).
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
; Demande de fermer l'application si elle tourne (mutex d'instance unique).
AppMutex=Local\ScreenCapture.Rust.SingleInstance
SetupIconFile=..\assets\ScreenCapture.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
WizardStyle=modern
; Langue de Windows si elle est proposée, sinon choix de la langue.
ShowLanguageDialog=auto
Compression=lzma2/max
SolidCompression=yes
OutputDir=..\dist
OutputBaseFilename=ScreenCapture-Setup-{#AppVersion}

[Languages]
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "de"; MessagesFile: "compiler:Languages\German.isl"

[CustomMessages]
fr.FfmpegTask=Télécharger et installer FFmpeg (nécessaire pour l'enregistrement vidéo, ~100 Mo)
fr.FfmpegGroup=Enregistrement vidéo :
fr.StartupGroup=Démarrage :
fr.StartupTask=Lancer {#AppName} au démarrage de Windows
fr.FfmpegDownloading=Téléchargement de FFmpeg depuis gyan.dev…
fr.FfmpegInstalling=Installation de FFmpeg…
fr.FfmpegCancelled=Téléchargement de FFmpeg annulé.%n%nContinuer l'installation sans FFmpeg ?
fr.FfmpegFailed=Le téléchargement de FFmpeg a échoué :%n%1%n%nContinuer l'installation sans FFmpeg ?
fr.FfmpegInstallFailed=FFmpeg n'a pas pu être installé (code %1).%nL'enregistrement vidéo restera indisponible ; vous pourrez indiquer un ffmpeg.exe dans les paramètres.

en.FfmpegTask=Download and install FFmpeg (needed for video recording, ~100 MB)
en.FfmpegGroup=Video recording:
en.StartupGroup=Startup:
en.StartupTask=Launch {#AppName} when Windows starts
en.FfmpegDownloading=Downloading FFmpeg from gyan.dev…
en.FfmpegInstalling=Installing FFmpeg…
en.FfmpegCancelled=FFmpeg download cancelled.%n%nContinue the installation without FFmpeg?
en.FfmpegFailed=The FFmpeg download failed:%n%1%n%nContinue the installation without FFmpeg?
en.FfmpegInstallFailed=FFmpeg could not be installed (code %1).%nVideo recording will be unavailable; you can set an ffmpeg.exe in the settings.

es.FfmpegTask=Descargar e instalar FFmpeg (necesario para grabar vídeo, ~100 MB)
es.FfmpegGroup=Grabación de vídeo:
es.StartupGroup=Inicio:
es.StartupTask=Iniciar {#AppName} al arrancar Windows
es.FfmpegDownloading=Descargando FFmpeg desde gyan.dev…
es.FfmpegInstalling=Instalando FFmpeg…
es.FfmpegCancelled=Descarga de FFmpeg cancelada.%n%n¿Continuar la instalación sin FFmpeg?
es.FfmpegFailed=La descarga de FFmpeg ha fallado:%n%1%n%n¿Continuar la instalación sin FFmpeg?
es.FfmpegInstallFailed=No se ha podido instalar FFmpeg (código %1).%nLa grabación de vídeo no estará disponible; puede indicar un ffmpeg.exe en los ajustes.

de.FfmpegTask=FFmpeg herunterladen und installieren (für Videoaufnahmen nötig, ~100 MB)
de.FfmpegGroup=Videoaufnahme:
de.StartupGroup=Start:
de.StartupTask={#AppName} beim Windows-Start ausführen
de.FfmpegDownloading=FFmpeg wird von gyan.dev heruntergeladen…
de.FfmpegInstalling=FFmpeg wird installiert…
de.FfmpegCancelled=FFmpeg-Download abgebrochen.%n%nInstallation ohne FFmpeg fortsetzen?
de.FfmpegFailed=Der FFmpeg-Download ist fehlgeschlagen:%n%1%n%nInstallation ohne FFmpeg fortsetzen?
de.FfmpegInstallFailed=FFmpeg konnte nicht installiert werden (Code %1).%nVideoaufnahmen bleiben nicht verfügbar; Sie können in den Einstellungen eine ffmpeg.exe angeben.

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startup"; Description: "{cm:StartupTask}"; GroupDescription: "{cm:StartupGroup}"; Flags: unchecked
Name: "ffmpeg"; Description: "{cm:FfmpegTask}"; GroupDescription: "{cm:FfmpegGroup}"; Check: not FfmpegInstalled

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon
Name: "{autostartup}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: startup

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{app}\ffmpeg"

[Code]
var
  DownloadPage: TDownloadWizardPage;
  FfmpegReady: Boolean;

{ Vrai si un sous-dossier de Base contient le fichier Rel (ex. '\bin\ffmpeg.exe'). }
function FileInSubdirs(const Base, Rel: String): Boolean;
var
  R: TFindRec;
begin
  Result := False;
  if FindFirst(AddBackslash(Base) + '*', R) then
  try
    repeat
      if ((R.Attributes and FILE_ATTRIBUTE_DIRECTORY) <> 0) and (R.Name <> '.') and (R.Name <> '..') then
        Result := FileExists(AddBackslash(Base) + R.Name + Rel);
    until Result or not FindNext(R);
  finally
    FindClose(R);
  end;
end;

{ Mêmes emplacements que recorder::find_ffmpeg dans l'application. }
function FfmpegInstalled: Boolean;
var
  R: TFindRec;
  Pkgs: String;
begin
  Result := (FileSearch('ffmpeg.exe', GetEnv('PATH')) <> '')
    or FileExists(AddBackslash(WizardDirValue) + 'ffmpeg.exe')
    or FileExists(AddBackslash(WizardDirValue) + 'ffmpeg\ffmpeg.exe')
    or FileExists(ExpandConstant('{localappdata}\Microsoft\WinGet\Links\ffmpeg.exe'))
    or FileExists('C:\ffmpeg\bin\ffmpeg.exe')
    or FileExists(ExpandConstant('{commonpf64}\ffmpeg\bin\ffmpeg.exe'));
#ifdef ForceFfmpeg
  { Test : propose FFmpeg même s'il est déjà présent (ISCC /DForceFfmpeg). }
  Result := False;
  Exit;
#endif
  if not Result then
  begin
    Pkgs := ExpandConstant('{localappdata}\Microsoft\WinGet\Packages\');
    if FindFirst(Pkgs + '*ffmpeg*', R) then
    try
      repeat
        Result := FileInSubdirs(Pkgs + R.Name, '\bin\ffmpeg.exe');
      until Result or not FindNext(R);
    finally
      FindClose(R);
    end;
  end;
end;

procedure InitializeWizard;
begin
  DownloadPage := CreateDownloadPage(SetupMessage(msgWizardPreparing), CustomMessage('FfmpegDownloading'), nil);
end;

{ Empreinte SHA-256 publiée à côté de l'archive ; vide si indisponible. }
function FfmpegSha256: String;
var
  S: AnsiString;
begin
  Result := '';
  try
    if DownloadTemporaryFile('{#FfmpegUrl}.sha256', 'ffmpeg.zip.sha256', '', nil) > 0 then
      if LoadStringFromFile(ExpandConstant('{tmp}\ffmpeg.zip.sha256'), S) then
        Result := Lowercase(Trim(Copy(String(S), 1, 64)));
  except
    Result := '';
  end;
  if Length(Result) <> 64 then
    Result := '';
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  if (CurPageID = wpReady) and WizardIsTaskSelected('ffmpeg') then
  begin
    DownloadPage.Clear;
    DownloadPage.Add('{#FfmpegUrl}', 'ffmpeg.zip', FfmpegSha256);
    DownloadPage.Show;
    try
      try
        DownloadPage.Download;
        FfmpegReady := True;
      except
        if DownloadPage.AbortedByUser then
          Result := MsgBox(CustomMessage('FfmpegCancelled'), mbConfirmation, MB_YESNO) = IDYES
        else
          Result := MsgBox(FmtMessage(CustomMessage('FfmpegFailed'), [GetExceptionMessage]), mbConfirmation, MB_YESNO) = IDYES;
      end;
    finally
      DownloadPage.Hide;
    end;
  end;
end;

{ Chaîne littérale PowerShell entre apostrophes. }
function PsQuote(const S: String): String;
begin
  Result := S;
  StringChangeEx(Result, '''', '''''', True);
  Result := '''' + Result + '''';
end;

procedure InstallFfmpeg;
var
  Zip, Tmp, Dest, Cmd: String;
  Code: Integer;
begin
  WizardForm.StatusLabel.Caption := CustomMessage('FfmpegInstalling');
  Zip := ExpandConstant('{tmp}\ffmpeg.zip');
  Tmp := ExpandConstant('{tmp}\ffmpeg');
  Dest := ExpandConstant('{app}\ffmpeg');
  ForceDirectories(Dest);
  { tar (intégré à Windows 10+) est bien plus rapide qu'Expand-Archive, gardé en secours. }
  Cmd := '$ErrorActionPreference = ''Stop''; ' +
    'New-Item -ItemType Directory -Force -Path ' + PsQuote(Tmp) + ' | Out-Null; ' +
    'tar.exe -xf ' + PsQuote(Zip) + ' -C ' + PsQuote(Tmp) + '; ' +
    'if ($LASTEXITCODE -ne 0) { Expand-Archive -LiteralPath ' + PsQuote(Zip) + ' -DestinationPath ' + PsQuote(Tmp) + ' -Force }; ' +
    '$f = Get-ChildItem -LiteralPath ' + PsQuote(Tmp) + ' -Recurse -Filter ffmpeg.exe | Select-Object -First 1; ' +
    'Copy-Item -LiteralPath $f.FullName -Destination ' + PsQuote(Dest) + ' -Force';
  if not Exec('powershell.exe', '-NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "' + Cmd + '"', '', SW_HIDE, ewWaitUntilTerminated, Code)
     or (Code <> 0) or not FileExists(Dest + '\ffmpeg.exe') then
    MsgBox(FmtMessage(CustomMessage('FfmpegInstallFailed'), [IntToStr(Code)]), mbError, MB_OK);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and FfmpegReady then
    InstallFfmpeg;
end;
