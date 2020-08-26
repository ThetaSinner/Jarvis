# Jarvis
Your helpful assistant for everything CI

### Setup environment

The CLI uses ANSI colours which are disabled by default on Windows. The following PowerShell command can be used to enable colour.

`Set-ItemProperty HKCU:\Console VirtualTerminalLevel -Type DWORD 1`

### Directory structure

```
/build
|-- /secrets
    `-- api-key
`-- /workspace
    |-- main.go
    `-- go.mod
```
