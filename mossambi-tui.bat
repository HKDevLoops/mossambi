@echo off
setlocal
set "BIN=%~dp0target\release\mossambi-tui.exe"
if not exist "%BIN%" set "BIN=%~dp0target\debug\mossambi-tui.exe"
"%BIN%" %*
