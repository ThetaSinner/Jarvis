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

### Feature wishlist

If this were going to be a fully featured CI system is would have...

- Tests against containers that were built during the CI process.
- Plugins for the build file which bring a managed container and actions such as container builds.
- Ability to run sidecar containers which interact with the main build container using lifecycle hooks.
- Ability to run service containers with managed lifecycles for testing.
- A default credentials management service which can be easily replaced by a custom one.
- Network monitoring to report what remote services the build used.
- Tool integrations (Codecov, GitHub, SonarQube, etc) as plugins.
- Manage variables between steps to allow features such as conditional steps.
- Control over parallelism.
- Build and step timeouts.
- Source control integration with customisation.
- Build matrices to support
    - Building on different systems
    - Testing on different systems
    - Testing against different services and service versions
- Build step implementations to be able to integrate with the build system to
    - Control service lifecycle
    - Report detailed build status
- Encryption and data destruction guarantees.
- Notifications as plugins to integrate with different services.