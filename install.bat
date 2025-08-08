@echo off
set SERVICE_NAME=AutoScreenSwitch
set EXE_PATH=%~dp0auto_screen_switch.exe

sc create %SERVICE_NAME% binPath= "%EXE_PATH%" start= auto
sc start %SERVICE_NAME%
