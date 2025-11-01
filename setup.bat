@echo off
:: === Configuration ===
set "REPO_URL=https://github.com/femn0x/Payjar.git"
set "CLONE_DIR=%USERPROFILE%\Documents\GitClones"

:: === Script ===
echo.
echo === GitHub Repo Cloner ===
echo.

:: Check if Git is installed
where git >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Git is not installed or not in PATH.
    echo Please install Git from https://git-scm.com/downloads
    pause
    exit /b
)

:: Create target directory if it doesn't exist
if not exist "%CLONE_DIR%" (
    echo Creating folder: %CLONE_DIR%
    mkdir "%CLONE_DIR%"
)

:: Change to target directory
cd /d "%CLONE_DIR%"

:: Clone the repository
echo Cloning repository from: %REPO_URL%
git clone "%REPO_URL%"

if %errorlevel% equ 0 (
    echo.
    echo Repository cloned successfully to:
    echo %CLONE_DIR%
) else (
    echo.
    echo  Failed to clone repository. Check the URL or your network.
)

echo.
pause
