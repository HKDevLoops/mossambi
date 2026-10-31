@echo off
setlocal
set "BIN=%~dp0target\release\mossambi.exe"
if not exist "%BIN%" set "BIN=%~dp0target\debug\mossambi.exe"
"%BIN%" %*
