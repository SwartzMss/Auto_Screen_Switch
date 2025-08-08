@echo off
set SERVICE_NAME=AutoScreenSwitch

sc stop %SERVICE_NAME%
sc delete %SERVICE_NAME%
