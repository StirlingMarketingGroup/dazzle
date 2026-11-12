# Set up Dazzle on a shared Windows PC

Use this guide for a Windows PC that several people sign in to, such as a shipping desk. It covers one PC set up by hand and many PCs set up through Group Policy (GPO).

Every command here is PowerShell. Run it as an administrator. Paste the commands into the PowerShell window. Do not save them as a `.ps1` file, because Windows may refuse to run a script file that is not signed.

## How Dazzle works on a shared PC

- **One Dazzle prints for the whole PC.** Each signed-in user runs their own Dazzle, but they all share one port. The first Dazzle to start serves print jobs. The others show "Standing by".
- **A standby takes over.** When the user whose Dazzle is serving signs out, a standby Dazzle takes the port within a few seconds.
- **Dazzle picks the label printer.** A print job goes to the first printer whose name or driver says it is a Zebra, Bixolon or ZPL printer. Every user who has not chosen a printer in the window gets the same pick, so the label prints the same way no matter whose Dazzle is serving. [Choose the printer](#choose-the-printer) covers the exceptions.

## The update command

Both setups below use this command. Paste it first, in the PowerShell window where you run the setup. Pasting it installs nothing. It only stores the command in `$update` and `$encoded`, and the setup uses those.

```powershell
$update = {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    $releases = 'https://github.com/StirlingMarketingGroup/dazzle/releases'
    # The "latest" address redirects to the newest release, such as .../releases/tag/v0.2.6.
    $version = [Net.WebRequest]::Create("$releases/latest").GetResponse().ResponseUri.Segments[-1].TrimStart('v')
    $installed = Get-ChildItem 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall' |
        Where-Object { $_.GetValue('DisplayName') -eq 'Dazzle' } |
        ForEach-Object { $_.GetValue('DisplayVersion') }
    if ($installed -eq $version) { return }

    $arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'arm64' } else { 'amd64' }
    # A new random name, so no other user of the PC can put their own file there first.
    $msi = Join-Path $env:TEMP "dazzle-$(New-Guid).msi"
    Invoke-WebRequest "$releases/download/v$version/dazzle_${version}_windows_$arch.msi" -OutFile $msi -UseBasicParsing
    $exit = (Start-Process msiexec -ArgumentList "/i `"$msi`" /qn /norestart" -Wait -PassThru).ExitCode
    Remove-Item $msi
    # 3010 means the install worked and Windows wants a restart.
    if ($exit -notin 0, 3010) { throw "The Dazzle installer failed with exit code $exit." }
}
$encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($update.ToString()))
```

What to know about it:

- **It installs the latest release from GitHub, or updates an older install.** It does nothing when the PC already has the latest version.
- **A scheduled task named "Dazzle update" runs it as SYSTEM.** A standard user cannot update Dazzle, because the installer needs administrator rights.
- **The task checks once a day, at 3:00.** A PC that is off or asleep at that time checks soon after it next starts.
- **The PC needs to reach `github.com` and `release-assets.githubusercontent.com`.** A check that fails changes nothing. The next day's check tries again.
- **The installer is not code-signed.** The command trusts whatever this repository publishes as its latest release.
- **A silent install does not start Dazzle.** Dazzle starts when a user next signs in.
- **It manages the install made with the `.msi` installer.** A copy that a user installed for themselves with the `_setup.exe` installer stays beside it and does not update. Uninstall that copy in that user's account.
- **Your security software may flag the task.** The task runs an encoded PowerShell command as SYSTEM. The encoded text is the command above.

## Set up one PC

Run this on the PC. Paste the update command above first, in the same window.

```powershell
# 1. Install the latest Dazzle now.
& $update

# 2. Check for a new version once a day.
Register-ScheduledTask -TaskName 'Dazzle update' -Force -User 'SYSTEM' -RunLevel Highest `
    -Trigger (New-ScheduledTaskTrigger -Daily -At 3am) `
    -Settings (New-ScheduledTaskSettingsSet -StartWhenAvailable -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries) `
    -Action (New-ScheduledTaskAction -Execute 'powershell.exe' -Argument "-NoProfile -ExecutionPolicy Bypass -EncodedCommand $encoded") |
    Out-Null

# 3. Start Dazzle for every user when they sign in.
[Microsoft.Win32.Registry]::SetValue(
    'HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer\Run',
    'Dazzle', '"C:\Program Files\Dazzle\dazzle.exe" --hidden')
```

Step 3 is the registry key behind the Group Policy setting "Run these programs at user logon". Keep `--hidden` in the value. Windows reads it at each sign-in. In Dazzle, "Launch at login" then shows as on for all users, and a user cannot turn it off.

The label printer has to be installed for the PC, not mapped by one user, or other users' Dazzle cannot reach it.

To stop the updates, run `Unregister-ScheduledTask -TaskName 'Dazzle update' -Confirm:$false`.

## Set up many PCs with Group Policy

One GPO does the whole setup. It adds the update task to every PC and starts Dazzle for every user. It names no printer, so it fits PCs with different label printers.

Run this on a computer that has the Group Policy Management tools, as a user who may create and link GPOs. A domain controller works. That computer does not need Dazzle, and nothing here installs Dazzle on it.

Paste the [update command](#the-update-command) first, in the same window. The GPO carries that command to each PC, and each PC runs it.

Set the two values at the top, then create the GPO.

```powershell
# Change these two values.
$gpoName = 'Dazzle'                     # No other GPO in your domain may have this name.
$ou = 'OU=Shipping,DC=example,DC=com'   # The OU that holds the PCs.

Import-Module GroupPolicy
$gpo = New-GPO -Name $gpoName
```

If that shows an error, stop and fix it before you go on. Then fill the GPO and link it.

```powershell
# 1. Add the "Dazzle update" task to every PC.
# The Group Policy commands work on this domain controller. Step 1 writes to the same one.
$dc = [DirectoryServices.ActiveDirectory.Domain]::GetCurrentDomain().PdcRoleOwner.Name
$folder = "\\$dc\SYSVOL\$($gpo.DomainName)\Policies\{$($gpo.Id)}"
New-Item "$folder\Machine\Preferences\ScheduledTasks" -ItemType Directory -Force | Out-Null
@"
<?xml version="1.0" encoding="utf-8"?>
<ScheduledTasks clsid="{CC63F200-7309-4ba0-B154-A71CD118DBCC}">
  <TaskV2 clsid="{D8896631-B747-47a7-84A6-C155337F3BC8}" name="Dazzle update" image="2" changed="$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" uid="{$(New-Guid)}">
    <Properties action="U" name="Dazzle update" runAs="NT AUTHORITY\System" logonType="S4U">
      <Task version="1.2">
        <RegistrationInfo>
          <Author>NT AUTHORITY\System</Author>
          <Description>Installs the latest Dazzle release.</Description>
        </RegistrationInfo>
        <Principals>
          <Principal id="Author">
            <UserId>NT AUTHORITY\System</UserId>
            <LogonType>S4U</LogonType>
            <RunLevel>HighestAvailable</RunLevel>
          </Principal>
        </Principals>
        <Settings>
          <IdleSettings>
            <Duration>PT10M</Duration>
            <WaitTimeout>PT1H</WaitTimeout>
            <StopOnIdleEnd>false</StopOnIdleEnd>
            <RestartOnIdle>false</RestartOnIdle>
          </IdleSettings>
          <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
          <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
          <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
          <AllowHardTerminate>true</AllowHardTerminate>
          <AllowStartOnDemand>true</AllowStartOnDemand>
          <Enabled>true</Enabled>
          <Hidden>false</Hidden>
          <ExecutionTimeLimit>PT1H</ExecutionTimeLimit>
          <Priority>7</Priority>
          <StartWhenAvailable>true</StartWhenAvailable>
        </Settings>
        <Triggers>
          <RegistrationTrigger>
            <Enabled>true</Enabled>
          </RegistrationTrigger>
          <CalendarTrigger>
            <StartBoundary>2026-01-01T03:00:00</StartBoundary>
            <Enabled>true</Enabled>
            <ScheduleByDay>
              <DaysInterval>1</DaysInterval>
            </ScheduleByDay>
          </CalendarTrigger>
        </Triggers>
        <Actions Context="Author">
          <Exec>
            <Command>powershell.exe</Command>
            <Arguments>-NoProfile -ExecutionPolicy Bypass -EncodedCommand $encoded</Arguments>
          </Exec>
        </Actions>
      </Task>
    </Properties>
  </TaskV2>
</ScheduledTasks>
"@ | Set-Content "$folder\Machine\Preferences\ScheduledTasks\ScheduledTasks.xml" -Encoding UTF8

# Tell Windows that the GPO holds a scheduled task, and that the GPO changed.
$ad = [adsi]"LDAP://$dc/$($gpo.Path)"
$ad.Put('gPCMachineExtensionNames', '[{00000000-0000-0000-0000-000000000000}{CAB54552-DEEA-4691-817E-ED4A4D1AFC72}][{AADCED64-746C-4633-A97C-D61349046527}{CAB54552-DEEA-4691-817E-ED4A4D1AFC72}]')
$ad.Put('versionNumber', 1)
$ad.SetInfo()
(Get-Content "$folder\GPT.INI") -replace '^Version=.*', 'Version=1' | Set-Content "$folder\GPT.INI"

# 2. Start Dazzle for every user when they sign in.
Set-GPRegistryValue -Guid $gpo.Id `
    -Key 'HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer\Run' `
    -ValueName 'Dazzle' -Type String -Value '"C:\Program Files\Dazzle\dazzle.exe" --hidden' | Out-Null

# 3. Apply the GPO to the PCs in the OU.
New-GPLink -Guid $gpo.Id -Target $ou | Out-Null
```

If step 3 shows an error, the OU is wrong. Set `$ou` again and run the step 3 line again. The GPO itself is complete.

What happens next:

1. Each PC gets the GPO within about two hours, or at its next restart.
2. The task installs Dazzle as soon as the PC has the GPO. After that, it checks for a new version once a day.
3. Each user gets Dazzle at their next sign-in.

What to know about the GPO:

- **Step 1 writes the task into the GPO's folder directly**, the way the Group Policy editor does. PowerShell has no command for a scheduled task in a GPO. The task shows in the editor under Computer Configuration → Preferences → Control Panel Settings → Scheduled Tasks.
- **Step 2 sets the same value as step 3 for one PC.** The note there applies.

### If an older GPO already deploys Dazzle

Take these two things out of it, in the Group Policy Management Editor, before you link the new GPO:

- **A Software Installation package** (Computer Configuration → Policies → Software Settings → Software installation). Remove it and choose "Allow users to continue to use the software". That package installs one fixed version, so it must not stay beside a task that installs the latest one.
- **A per-user `Run` entry** (User Configuration → Preferences → Windows Settings → Registry, under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`). Delete it. A GPO linked to an OU of computers applies its User Configuration only when loopback processing is on, so this entry often never reaches the users. The computer value above needs no loopback. An entry without `--hidden` also opens the Dazzle window at every sign-in.

## Choose the printer

Most PCs need nothing here. When a print job names no printer, Dazzle decides in this order:

1. The printer an administrator set for the PC, as shown below.
2. The printer chosen in the window of the Dazzle that is serving, when it is still installed.
3. The first printer, by name, whose name or driver contains `Zebra`, `ZDesigner`, `Bixolon` or `ZPL`. A printer that Windows shows as offline comes last. In the window this is "Automatic", and the window shows the printer it picked.
4. When no printer matches, the default printer of the user whose Dazzle is serving, or the first printer when that user has no default.

Set the printer yourself on a PC with more than one label printer, or with a label printer that step 3 does not recognize. Run this on the PC:

```powershell
# Change this value.
$printer = 'ZDesigner ZD420'   # The printer's exact Windows name. Get-Printer lists the names.

[Microsoft.Win32.Registry]::SetValue(
    'HKEY_LOCAL_MACHINE\SOFTWARE\Policies\StirlingMarketingGroup\Dazzle',
    'Printer', $printer)
```

- **Dazzle locks its printer list to this printer** for every user. It reads the value for every print job, so a change needs no restart of Dazzle.
- **A web app that names a printer in its own request (`?printer=`) still gets that printer.**
- **To let Dazzle pick again**, run `Remove-ItemProperty 'HKLM:\SOFTWARE\Policies\StirlingMarketingGroup\Dazzle' -Name Printer`.

A GPO can set the same value. It gives every PC the same name, so use one only where every PC has a printer with that name.

```powershell
# Change these two values.
$gpoName = 'Dazzle'            # Your GPO's name.
$printer = 'ZDesigner ZD420'   # The printer's exact Windows name on every PC.

Set-GPRegistryValue -Name $gpoName `
    -Key 'HKLM\SOFTWARE\Policies\StirlingMarketingGroup\Dazzle' `
    -ValueName 'Printer' -Type String -Value $printer | Out-Null
```

To take the printer out of the GPO again, run this. Each PC lets Dazzle pick again after its next policy refresh.

```powershell
# Change this value.
$gpoName = 'Dazzle'   # Your GPO's name.

Remove-GPRegistryValue -Name $gpoName `
    -Key 'HKLM\SOFTWARE\Policies\StirlingMarketingGroup\Dazzle' -ValueName 'Printer' | Out-Null
```

Dazzle 0.2.7 and older saved a printer by itself. When a newer Dazzle starts on a PC that has a printer step 3 recognizes, it forgets a saved printer that is not one. You do not need to do anything.

## Check that it worked

Restart the PC, sign in as two different users, and print a label as each.

| Question                                             | Command                                                                                                  |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Who is signed in?                                    | `query user`                                                                                             |
| Is a Dazzle serving, and which version?              | `Invoke-RestMethod http://127.0.0.1:29100/status`                                                        |
| Whose Dazzle is serving? (as administrator)          | `Get-Process -Id (Get-NetTCPConnection -LocalPort 29100 -State Listen).OwningProcess -IncludeUserName`   |
| Is the start-at-sign-in value set?                   | `Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer\Run'`               |
| Did an administrator set a printer? (no output = no) | `Get-ItemProperty 'HKLM:\SOFTWARE\Policies\StirlingMarketingGroup\Dazzle' -ErrorAction SilentlyContinue` |
| Did the update task run? (`LastTaskResult` 0 = ok)   | `Get-ScheduledTaskInfo 'Dazzle update'`                                                                  |
| Did the GPO reach this PC?                           | `gpresult /r /scope computer`                                                                            |
| What did this user's Dazzle do?                      | `Get-Content "$env:LOCALAPPDATA\com.stirlingmarketinggroup.dazzle\logs\Dazzle.log" -Tail 20`             |

## What users see

| In the Dazzle window or tray                                                              | Meaning                                                                                                                                                                 |
| ----------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Green badge, "Server running"                                                             | This user's Dazzle prints for the PC.                                                                                                                                   |
| Green badge, "Standing by"                                                                | Another signed-in user's Dazzle prints for the PC. Print jobs show in that user's window.                                                                               |
| Red badge, "Not printing", and a "Dazzle can't print" notification                        | No Dazzle can print. The message says why. "Port 29100 is in use by another program" means a program that is not Dazzle holds the port.                                 |
| Red text, "Your administrator chose the printer …, but it is not installed for this user" | Windows has no printer with the name an administrator set. Labels fail until you set the name again or remove the value. See [Choose the printer](#choose-the-printer). |

## After an update

An update does not stop a Dazzle that is already running. That Dazzle keeps the old version until its user signs out, and Windows lists it under a name that ends in `.rbf`, not `dazzle.exe`. Every user who signs in after the update gets the new version. A restart puts the whole PC on the new version.
