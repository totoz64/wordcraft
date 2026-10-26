@echo off
setlocal DisableDelayedExpansion
set "PREVIEW_ROOT=%~dp0"
set "WORDCRAFT_PROFILE_DIR=%PREVIEW_ROOT%profile"
set "WORDCRAFT_DICTIONARIES=%PREVIEW_ROOT%dictionaries"
set "WORDCRAFT_LANGUAGE=fr"
set "WORDCRAFT_REGION=fr-FR"
set "WORDCRAFT_CONTROL_PORT="
set "WORDCRAFT_NO_PREFS="
if not exist "%PREVIEW_ROOT%wordcraft.exe" goto missing
if not exist "%PREVIEW_ROOT%demo\essai-francais.docx" goto missing
if not exist "%WORDCRAFT_PROFILE_DIR%\" (
  mkdir "%WORDCRAFT_PROFILE_DIR%"
  if errorlevel 1 goto failed
)
if not exist "%WORDCRAFT_DICTIONARIES%\" (
  mkdir "%WORDCRAFT_DICTIONARIES%"
  if errorlevel 1 goto failed
)
where.exe curl.exe >nul 2>&1
if errorlevel 1 goto tools
where.exe certutil.exe >nul 2>&1
if errorlevel 1 goto tools
set "DICT_BASE=https://raw.githubusercontent.com/LibreOffice/dictionaries/67032ee957631908098a72edf48dc81cac27e369/fr_FR/dictionaries"
call :dictionary fr.aff c176610cd5dc4846806a65ddd029f422d87978bf58f224aa44222662a16a2de5
if errorlevel 1 goto failed
call :dictionary fr.dic b78a868e31dd6e373b6c3217969afb898a9acde828a5e7ef97308da42218c88c
if errorlevel 1 goto failed
if not exist "%WORDCRAFT_DICTIONARIES%\README_dict_fr.txt" (
  curl.exe --proto "=https" --tlsv1.2 --location --fail --show-error --output "%WORDCRAFT_DICTIONARIES%\README_dict_fr.txt.download" "%DICT_BASE%/README_dict_fr.txt"
  if errorlevel 1 goto failed
  move /Y "%WORDCRAFT_DICTIONARIES%\README_dict_fr.txt.download" "%WORDCRAFT_DICTIONARIES%\README_dict_fr.txt" >nul
  if errorlevel 1 goto failed
)
echo Lancement de la preversion WordCraft avec un profil separe.
echo Dans le document, appuyez sur F7 pour ouvrir le volet de verification.
start "" /D "%PREVIEW_ROOT%" "%PREVIEW_ROOT%wordcraft.exe" "%PREVIEW_ROOT%demo\essai-francais.docx"
if errorlevel 1 goto failed
exit /b 0

:dictionary
if exist "%WORDCRAFT_DICTIONARIES%\%~1" (
  certutil.exe -hashfile "%WORDCRAFT_DICTIONARIES%\%~1" SHA256 2>nul | findstr.exe /I /X /C:"%~2" >nul
  if not errorlevel 1 exit /b 0
)
echo Preparation du dictionnaire francais : %~1
curl.exe --proto "=https" --tlsv1.2 --location --fail --show-error --output "%WORDCRAFT_DICTIONARIES%\%~1.download" "%DICT_BASE%/%~1"
if errorlevel 1 exit /b 1
certutil.exe -hashfile "%WORDCRAFT_DICTIONARIES%\%~1.download" SHA256 2>nul | findstr.exe /I /X /C:"%~2" >nul
if errorlevel 1 (
  echo ERREUR : empreinte SHA-256 du dictionnaire incorrecte.
  exit /b 1
)
move /Y "%WORDCRAFT_DICTIONARIES%\%~1.download" "%WORDCRAFT_DICTIONARIES%\%~1" >nul
exit /b %ERRORLEVEL%

:missing
echo ERREUR : archive incomplete. Extrayez tout le ZIP avant de lancer ce fichier.
goto failed
:tools
echo ERREUR : curl.exe et certutil.exe sont necessaires sur ce Windows.
goto failed
:failed
echo La preversion n'a pas ete lancee. Consultez README-francais.txt.
echo Aucun correcteur francais inactif ne sera lance silencieusement.
pause
exit /b 1
