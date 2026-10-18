<#
    Tariqi - School PC Survey
    =========================
    Read-only survey of a driving school's office PC.
    Answers open question Q3 (Windows version / stack viability) and Q8 (printers).

    HOW TO USE
      1. Copy this file onto a USB stick.
      2. On the school's PC, right-click it -> "Run with PowerShell".
         If that is blocked, open PowerShell and run:
             powershell -ExecutionPolicy Bypass -File .\survey-pc.ps1
      3. A report .txt appears next to this script. Collect it and send it back.

    This script only READS. It changes nothing, installs nothing, and sends nothing.
    Written to run on PowerShell 2.0 so that it works on old Windows too -
    detecting old Windows is the whole point, so it must not require a new one.
#>

# ---------------------------------------------------------------- helpers ----

$script:Lines = @()

function Add-Line {
    param([string]$Text = "")
    $script:Lines += $Text
    Write-Host $Text
}

function Add-Header {
    param([string]$Title)
    Add-Line ""
    Add-Line ("--- " + $Title + " " + ("-" * [Math]::Max(0, 60 - $Title.Length)))
}

function Get-Wmi {
    param([string]$Class, [string]$Namespace = "root\cimv2")
    try   { return Get-WmiObject -Class $Class -Namespace $Namespace -ErrorAction Stop }
    catch { return $null }
}

function Get-RegValue {
    param([string]$Path, [string]$Name)
    try {
        $item = Get-ItemProperty -Path $Path -ErrorAction Stop
        return $item.$Name
    } catch { return $null }
}

function Format-GB {
    param($Bytes)
    if ($Bytes -eq $null) { return "unknown" }
    return ("{0:N1} GB" -f ($Bytes / 1GB))
}

# ------------------------------------------------------------- identity -----

Add-Line "=============================================================="
Add-Line " TARIQI - SCHOOL PC SURVEY"
Add-Line (" Run at : " + (Get-Date -Format "yyyy-MM-dd HH:mm:ss"))
Add-Line (" PC     : " + $env:COMPUTERNAME)
Add-Line (" User   : " + $env:USERNAME)
Add-Line "=============================================================="

# ------------------------------------------------------------- windows ------

Add-Header "WINDOWS"

$os    = Get-Wmi "Win32_OperatingSystem"
$build = 0
$major = 0
$minor = 0

if ($os -ne $null) {
    Add-Line ("Edition      : " + $os.Caption)
    Add-Line ("Version      : " + $os.Version)
    Add-Line ("Build        : " + $os.BuildNumber)
    Add-Line ("Architecture : " + $os.OSArchitecture)
    Add-Line ("Service Pack : " + $os.ServicePackMajorVersion)
    Add-Line ("Locale ID    : " + $os.OSLanguage)

    $build = [int]$os.BuildNumber
    $parts = $os.Version.Split(".")
    $major = [int]$parts[0]
    $minor = [int]$parts[1]
} else {
    Add-Line "Could not read OS info via WMI."
    $v = [System.Environment]::OSVersion.Version
    $major = $v.Major; $minor = $v.Minor; $build = $v.Build
    Add-Line ("Fallback version : " + $v.ToString())
}

Add-Line ("PowerShell   : " + $PSVersionTable.PSVersion.ToString())

# ------------------------------------------------------- the key verdict ----

Add-Header "VERDICT: CAN THIS PC RUN A TAURI / WEBVIEW2 APP?"

$verdict = "UNKNOWN"
if ($build -ge 22000) {
    $verdict = "YES - Windows 11. Fully supported."
} elseif ($major -eq 10) {
    if ($build -ge 17763) {
        $verdict = "YES - Windows 10 (1809 or later). Fully supported."
    } else {
        $verdict = "PROBABLY - Windows 10 but an old build ($build). Update recommended."
    }
} elseif ($major -eq 6 -and $minor -eq 3) {
    $verdict = "NO - Windows 8.1. WebView2 dropped support. Stack must change."
} elseif ($major -eq 6 -and $minor -eq 2) {
    $verdict = "NO - Windows 8. Not supported. Stack must change."
} elseif ($major -eq 6 -and $minor -eq 1) {
    $verdict = "NO - Windows 7. Not supported. Stack must change (.NET + WPF)."
} else {
    $verdict = "NO - Windows version too old ($major.$minor build $build)."
}

Add-Line $verdict

# ------------------------------------------------------------- webview2 -----

Add-Header "WEBVIEW2 RUNTIME"

$wvGuid  = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
$wvPaths = @(
    ("HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\" + $wvGuid),
    ("HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\" + $wvGuid),
    ("HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\" + $wvGuid)
)

$wvVersion = $null
foreach ($p in $wvPaths) {
    if ($wvVersion -eq $null) {
        $v = Get-RegValue $p "pv"
        if ($v -ne $null -and $v -ne "" -and $v -ne "0.0.0.0") { $wvVersion = $v }
    }
}

if ($wvVersion -ne $null) {
    Add-Line ("Installed    : YES, version " + $wvVersion)
    Add-Line "Note         : no extra download needed at install time."
} else {
    Add-Line "Installed    : NO"
    Add-Line "Note         : the installer must bundle the offline WebView2 bootstrapper."
}

# --------------------------------------------------------------- dotnet -----

Add-Header ".NET (relevant only if the stack has to fall back to WPF)"

$ndpVersion = Get-RegValue "HKLM:\SOFTWARE\Microsoft\NET Framework Setup\NDP\v4\Full" "Version"
if ($ndpVersion -ne $null) {
    Add-Line (".NET Framework 4.x : " + $ndpVersion)
} else {
    Add-Line ".NET Framework 4.x : not detected"
}

try {
    $runtimes = & dotnet --list-runtimes 2>$null
    if ($runtimes) {
        Add-Line ".NET (modern)      :"
        foreach ($r in $runtimes) { Add-Line ("  " + $r) }
    } else {
        Add-Line ".NET (modern)      : dotnet CLI not present"
    }
} catch {
    Add-Line ".NET (modern)      : dotnet CLI not present"
}

# ------------------------------------------------------------- hardware -----

Add-Header "HARDWARE"

$cs  = Get-Wmi "Win32_ComputerSystem"
$cpu = Get-Wmi "Win32_Processor"

if ($cs -ne $null) {
    Add-Line ("Model        : " + $cs.Manufacturer + " " + $cs.Model)
    Add-Line ("RAM          : " + (Format-GB $cs.TotalPhysicalMemory))
    if ($cs.TotalPhysicalMemory -lt 4GB) {
        Add-Line "  WARNING    : under 4 GB. Performance target must account for this."
    }
}

if ($cpu -ne $null) {
    # $cpu may be an array on multi-socket machines
    $c0 = $cpu
    if ($cpu -is [Array]) { $c0 = $cpu[0] }
    Add-Line ("CPU          : " + $c0.Name.Trim())
    Add-Line ("Cores        : " + $c0.NumberOfCores + " physical / " + $c0.NumberOfLogicalProcessors + " logical")
}

$sysDrive = $env:SystemDrive
$disks = Get-Wmi "Win32_LogicalDisk"
if ($disks -ne $null) {
    foreach ($d in $disks) {
        if ($d.DriveType -eq 3) {
            $flag = ""
            if ($d.DeviceID -eq $sysDrive) { $flag = "  <- system drive" }
            Add-Line ("Disk " + $d.DeviceID + "       : " + (Format-GB $d.FreeSpace) + " free of " + (Format-GB $d.Size) + $flag)
        }
    }
}

# SSD vs HDD - only available on Windows 8 and later, so failure here is itself informative
$mediaType = "unknown (API not available - suggests older Windows)"
try {
    $phys = Get-WmiObject -Namespace "root\Microsoft\Windows\Storage" -Class "MSFT_PhysicalDisk" -ErrorAction Stop
    if ($phys -ne $null) {
        $types = @()
        foreach ($p in $phys) {
            switch ([int]$p.MediaType) {
                3 { $types += "HDD" }
                4 { $types += "SSD" }
                5 { $types += "SCM" }
                default { $types += "unspecified" }
            }
        }
        $mediaType = [string]::Join(", ", $types)
    }
} catch { }
Add-Line ("Disk type    : " + $mediaType)

$video = Get-Wmi "Win32_VideoController"
if ($video -ne $null) {
    $v0 = $video
    if ($video -is [Array]) { $v0 = $video[0] }
    if ($v0.CurrentHorizontalResolution) {
        Add-Line ("Screen       : " + $v0.CurrentHorizontalResolution + " x " + $v0.CurrentVerticalResolution)
        if ([int]$v0.CurrentHorizontalResolution -lt 1366) {
            Add-Line "  WARNING    : narrow screen. UI must work at this width."
        }
    }
}

# ------------------------------------------------------------- printers -----

Add-Header "PRINTERS (answers Q8 - do schools use thermal receipt printers?)"

$printers = Get-Wmi "Win32_Printer"
if ($printers -eq $null) {
    Add-Line "Could not enumerate printers."
} else {
    $thermalHints = @("POS", "THERMAL", "RECEIPT", "TM-T", "TM-U", "XP-", "ZJ-", "58MM", "80MM", "STAR ", "BIXOLON", "SEWOO", "RONGTA")
    $count = 0
    foreach ($p in $printers) {
        $count++
        $tag = ""
        if ($p.Default) { $tag = "  [DEFAULT]" }

        $upper = ($p.Name + " " + $p.DriverName).ToUpper()
        $isThermal = $false
        foreach ($h in $thermalHints) {
            if ($upper.Contains($h)) { $isThermal = $true }
        }
        if ($isThermal) { $tag = $tag + "  [LIKELY THERMAL / RECEIPT]" }

        Add-Line ("Printer      : " + $p.Name + $tag)
        Add-Line ("  Driver     : " + $p.DriverName)
        Add-Line ("  Port       : " + $p.PortName)
        if ($p.HorizontalResolution) {
            Add-Line ("  Resolution : " + $p.HorizontalResolution + " dpi")
        }
    }
    if ($count -eq 0) { Add-Line "No printers installed on this PC." }
}

# -------------------------------------------------------------- arabic ------

Add-Header "ARABIC SUPPORT"

try {
    $culture = Get-Culture
    Add-Line ("Current culture  : " + $culture.Name + " (" + $culture.DisplayName + ")")
} catch { }

$fontsDir = Join-Path $env:WINDIR "Fonts"
$arabicFonts = @("arial.ttf", "tahoma.ttf", "trado.ttf", "simpo.ttf", "NotoNaskhArabic-Regular.ttf")
$found = @()
foreach ($f in $arabicFonts) {
    if (Test-Path (Join-Path $fontsDir $f)) { $found += $f }
}
if ($found.Count -gt 0) {
    Add-Line ("Arabic-capable fonts present : " + [string]::Join(", ", $found))
} else {
    Add-Line "Arabic-capable fonts present : none of the expected ones found"
}
Add-Line "Note             : the app bundles its own Arabic font, so this is informational."

# ------------------------------------------------------------ internet ------

Add-Header "CONNECTIVITY (informs the cloud-sync add-on decision)"

$online = $false
try {
    $online = Test-Connection -ComputerName "8.8.8.8" -Count 2 -Quiet -ErrorAction SilentlyContinue
} catch { }

if ($online) {
    Add-Line "Internet     : reachable right now"
} else {
    Add-Line "Internet     : NOT reachable right now (or ICMP blocked)"
}
Add-Line "Ask the school: is there internet every day, and is it reliable?"

# --------------------------------------------------------------- notes ------

Add-Header "NOTES - fill these in by hand"

Add-Line "School name          : ______________________________________"
Add-Line "Wilaya               : ______________________________________"
Add-Line "How many PCs total   : ______________________________________"
Add-Line "Who uses this PC     : ______________________________________"
Add-Line "Is it shared/public  : ______________________________________"
Add-Line "Power cuts per week  : ______________________________________"
Add-Line "Is there a UPS       : ______________________________________"

# -------------------------------------------------------------- output ------

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
if (-not $scriptDir) { $scriptDir = (Get-Location).Path }

$stamp      = Get-Date -Format "yyyyMMdd-HHmmss"
$reportName = "Tariqi-Survey-" + $env:COMPUTERNAME + "-" + $stamp + ".txt"
$reportPath = Join-Path $scriptDir $reportName

try {
    $script:Lines | Out-File -FilePath $reportPath -Encoding utf8
    Write-Host ""
    Write-Host "==============================================================" -ForegroundColor Green
    Write-Host (" Report saved: " + $reportPath) -ForegroundColor Green
    Write-Host "==============================================================" -ForegroundColor Green
} catch {
    Write-Host ""
    Write-Host "Could not write the report file. Copy the text above by hand." -ForegroundColor Yellow
    Write-Host $_.Exception.Message -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Press Enter to close..."
try { Read-Host | Out-Null } catch { }
