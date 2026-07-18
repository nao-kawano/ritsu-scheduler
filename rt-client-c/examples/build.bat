@echo off
cd %~dp0
if not exist build mkdir build
cl /EHsc /I.. example_cpp.cpp /Fobuild/ /Febuild/example_cpp.exe
