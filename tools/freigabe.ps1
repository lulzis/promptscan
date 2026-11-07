<#
.SYNOPSIS
Baut die Release-Programmdatei ohne Entwicklerpfade, prueft sie und schreibt die SHA-256-Pruefsumme.

.DESCRIPTION
Windows PowerShell 5.1, nur ASCII. Aufruf im Projektordner:

    powershell -NoProfile -ExecutionPolicy Bypass -File tools\freigabe.ps1

Schritte (Einzelheiten in docs/WEITERGABE.md):
1. Pfade bestimmen: Projektordner, CARGO_HOME, RUSTUP_HOME, Benutzerprofil, Zielordner.
2. cargo build --release --locked mit --remap-path-prefix fuer diese Pfade. Die Flags gehen
   ueber "cargo --config" unter demselben Schluessel, den die Cargo-Konfiguration schon nutzt
   (target.<host>.rustflags oder build.rustflags); Cargo haengt Listen aus mehreren Quellen
   aneinander, die vorhandenen Flags bleiben also erhalten. RUSTFLAGS wuerde die Konfiguration
   ersetzen und ist darum nicht erlaubt.
3. Zaehlen: Kein Pfad aus Schritt 1, kein "C:\Users\" und nicht der Kontoname in Pfadform
   ("\<Konto>\", "/<Konto>/", "Users\<Konto>", "Users/<Konto>") duerfen in der Programmdatei
   stehen (UTF-8 und UTF-16LE, ohne Gross/klein). Sonst Abbruch mit Exit-Code 1. Den
   Kontonamen allein meldet das Skript nur: Die eingebetteten Namens- und Ortsregister koennen
   ihn als Wort enthalten. Die Ausgabe zeigt den Kontonamen als <Konto>.
4. Strenge Importpruefung: cargo test --test offline mit PROMPTSCAN_EXE auf die neue Datei.
5. SHA-256 in <Programmdatei>.sha256 (Format von sha256sum).
6. LICENSE.md und THIRD-PARTY-NOTICES.md neben die Programmdatei kopieren.

.PARAMETER Zielordner
Cargo-Zielordner (sonst CARGO_TARGET_DIR, sonst <Projekt>\target).

.PARAMETER Jobs
Anzahl paralleler Uebersetzungen fuer cargo (-j); 0 = Vorgabe von cargo.

.PARAMETER OhneImportpruefung
Schritt 4 auslassen.

.PARAMETER Selbsttest
Nur die Zaehlung aus Schritt 3 an einer kuenstlichen Probe pruefen, ohne Bau. Exit-Code 0 =
bestanden.
#>
[CmdletBinding()]
param(
    [string]$Zielordner = "",
    [int]$Jobs = 0,
    [switch]$OhneImportpruefung,
    [switch]$Selbsttest
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

function Abbruch([string]$Text) {
    Write-Host "freigabe: $Text"
    exit 1
}

function Volle-Pfad([string]$Pfad) {
    # Relativ zum aktuellen Ort von PowerShell, nicht zum Arbeitsordner des Prozesses.
    $voll = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Pfad)
    return [System.IO.Path]::GetFullPath($voll).TrimEnd('\')
}

# Die Programmdatei wird als Latin-1 gelesen, ein Zeichen je Byte. Jedes Muster wird in UTF-8
# gesucht (so legt Rust Pfade ab) und in UTF-16LE (so legen Windows-Funktionen Text ab).
$Latin1 = [System.Text.Encoding]::GetEncoding(28591)
$Ohne = [System.StringComparison]::OrdinalIgnoreCase
$Genau = [System.StringComparison]::Ordinal

function Zaehle([string]$Inhalt, [string]$Muster, [System.StringComparison]$Vergleich) {
    $n = 0
    foreach ($Kodierung in @([System.Text.Encoding]::UTF8, [System.Text.Encoding]::Unicode)) {
        $m = $Latin1.GetString($Kodierung.GetBytes($Muster))
        $i = $Inhalt.IndexOf($m, $Vergleich)
        while ($i -ge 0) {
            $n++
            $i = $Inhalt.IndexOf($m, $i + $m.Length, $Vergleich)
        }
    }
    return $n
}

# Der Kontoname in Pfadform. Allein (als Wort) zaehlt er nicht: Die eingebetteten Register
# enthalten Vor-, Nach- und Ortsnamen.
function Konto-Muster([string]$Konto) {
    return @("\$Konto\", "/$Konto/", "Users\$Konto", "Users/$Konto")
}

# Text fuer die Ausgabe: der Kontoname als <Konto>.
function Anzeige([string]$Text) {
    if (-not $env:USERNAME) { return $Text }
    return [regex]::Replace($Text, [regex]::Escape($env:USERNAME), '<Konto>', 'IgnoreCase')
}

if ($Selbsttest) {
    # Probe: ein Pfad mit Umlaut, einmal in UTF-8 und einmal in UTF-16LE.
    $Ort = 'Z' + [char]0xFC + 'rich'
    $Probe = "x C:\Users\$Ort\.cargo x"
    $Inhalt = $Latin1.GetString([byte[]]([System.Text.Encoding]::UTF8.GetBytes($Probe) +
        [System.Text.Encoding]::Unicode.GetBytes($Probe)))
    $Erwartet = @(
        @($Ort, $Genau, 2),
        @('c:\users\', $Ohne, 2),
        @('c:\users\', $Genau, 0),
        @('Bern', $Genau, 0)
    )
    foreach ($e in $Erwartet) {
        $n = Zaehle $Inhalt $e[0] $e[1]
        if ($n -ne $e[2]) { Abbruch "Selbsttest: '$($e[0])' $n-mal statt $($e[2])-mal gefunden." }
    }
    # Kontoname (erfunden: Muster): in Pfadform gefunden, als Wort in einem Register nicht.
    $Proben = @(
        @("x C:\Users\muster\.cargo x", @(1, 0, 1, 0)),
        @("x C:/Users/MUSTER/.cargo x", @(0, 1, 0, 1)),
        @("Muster`t1234`nMusterweg`t56`n", @(0, 0, 0, 0))
    )
    foreach ($p in $Proben) {
        $Inhalt = $Latin1.GetString([System.Text.Encoding]::UTF8.GetBytes($p[0]))
        $Muster = Konto-Muster 'Muster'
        for ($k = 0; $k -lt $Muster.Count; $k++) {
            $n = Zaehle $Inhalt $Muster[$k] $Ohne
            if ($n -ne $p[1][$k]) {
                Abbruch "Selbsttest: '$($Muster[$k])' in '$($p[0])' $n-mal statt $($p[1][$k])-mal gefunden."
            }
        }
    }
    if ($env:USERNAME -and (Anzeige "C:\Users\$($env:USERNAME.ToUpper())\x") -ne 'C:\Users\<Konto>\x') {
        Abbruch 'Selbsttest: Anzeige ersetzt den Kontonamen nicht.'
    }
    Write-Host 'freigabe: Selbsttest bestanden.'
    exit 0
}

# --- 1. Pfade -------------------------------------------------------------------------------

$Projekt = Volle-Pfad (Join-Path $PSScriptRoot '..')
if (-not (Test-Path -LiteralPath (Join-Path $Projekt 'Cargo.toml'))) {
    Abbruch "Cargo.toml nicht gefunden in $Projekt"
}
$Profil = Volle-Pfad $env:USERPROFILE
if ($env:CARGO_HOME) { $CargoHome = Volle-Pfad $env:CARGO_HOME } else { $CargoHome = Join-Path $Profil '.cargo' }
if ($env:RUSTUP_HOME) { $RustupHome = Volle-Pfad $env:RUSTUP_HOME } else { $RustupHome = Join-Path $Profil '.rustup' }
if (-not $Zielordner) {
    if ($env:CARGO_TARGET_DIR) { $Zielordner = $env:CARGO_TARGET_DIR } else { $Zielordner = Join-Path $Projekt 'target' }
}
$Zielordner = Volle-Pfad $Zielordner

foreach ($v in 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS') {
    if (Test-Path -LiteralPath "env:$v") {
        Abbruch "$v ist gesetzt und ersetzte die Cargo-Konfiguration samt --config; bitte leeren."
    }
}

$Wirt = ''
foreach ($zeile in (& rustc -vV)) {
    if ($zeile -match '^host:\s*(\S+)') { $Wirt = $Matches[1] }
}
if ($LASTEXITCODE -ne 0 -or -not $Wirt) { Abbruch 'rustc -vV ohne Angabe "host"' }

# Welcher Schluessel traegt die rustflags in der Cargo-Konfiguration? Gelesen werden die
# Konfigurationsdateien im Projektordner und darueber sowie in CARGO_HOME.
$Dateien = @()
$Ordner = $Projekt
while ($Ordner) {
    $Dateien += (Join-Path $Ordner '.cargo\config.toml'), (Join-Path $Ordner '.cargo\config')
    $Oben = Split-Path -Parent $Ordner
    if ($Oben -eq $Ordner) { break }
    $Ordner = $Oben
}
$Dateien += (Join-Path $CargoHome 'config.toml'), (Join-Path $CargoHome 'config')
$ZielFlags = $false
$BauFlags = $false
foreach ($d in $Dateien) {
    if (-not (Test-Path -LiteralPath $d)) { continue }
    $Abschnitt = ''
    foreach ($zeile in (Get-Content -LiteralPath $d)) {
        $z = $zeile.Trim()
        if ($z -match '^\[([^\]]+)\]') { $Abschnitt = $Matches[1].Trim(); continue }
        if ($z -match '^rustflags\s*=') {
            if ($Abschnitt -eq "target.$Wirt") { $ZielFlags = $true }
            if ($Abschnitt -eq 'build') { $BauFlags = $true }
        }
        if ($z -match "^target\.$([regex]::Escape($Wirt))\.rustflags\s*=") { $ZielFlags = $true }
        if ($z -match '^build\.rustflags\s*=') { $BauFlags = $true }
    }
}
if ($BauFlags -and -not $ZielFlags) { $Schluessel = 'build.rustflags' } else { $Schluessel = "target.$Wirt.rustflags" }

# Allgemeine Pfade zuerst: Bei mehreren passenden Abbildungen gilt die letzte.
$Abbildungen = @(
    @($Profil, '/home'),
    @($RustupHome, '/rustup'),
    @($CargoHome, '/cargo'),
    @($Projekt, '/promptscan'),
    @($Zielordner, '/target')
)
$Flags = @()
foreach ($a in $Abbildungen) {
    if ($a[0].Contains("'")) { Abbruch "Pfad mit Apostroph wird nicht unterstuetzt: $($a[0])" }
    $Flags += "'--remap-path-prefix=$($a[0])=$($a[1])'"
}
$Konfig = "$Schluessel=[" + ($Flags -join ',') + ']'

Write-Host "Projekt:     $(Anzeige $Projekt)"
Write-Host "Zielordner:  $(Anzeige $Zielordner)"
Write-Host "Schluessel:  $Schluessel"

# --- 2. Bau ---------------------------------------------------------------------------------

$JobsArg = @()
if ($Jobs -gt 0) { $JobsArg = @('-j', "$Jobs") }
$env:CARGO_TARGET_DIR = $Zielordner
Push-Location -LiteralPath $Projekt
try {
    & cargo --config $Konfig build --release --locked @JobsArg
    if ($LASTEXITCODE -ne 0) { Abbruch "cargo build endete mit $LASTEXITCODE" }
} finally {
    Pop-Location
}

# Name der Programmdatei: [[bin]] name, sonst [package] name.
$Name = ''
$Paketname = ''
$Abschnitt = ''
foreach ($zeile in (Get-Content -LiteralPath (Join-Path $Projekt 'Cargo.toml'))) {
    $z = $zeile.Trim()
    if ($z -match '^\[\[?([^\]]+)\]\]?') { $Abschnitt = $Matches[1]; continue }
    if ($z -match '^name\s*=\s*"([^"]+)"') {
        if ($Abschnitt -eq 'bin' -and -not $Name) { $Name = $Matches[1] }
        if ($Abschnitt -eq 'package' -and -not $Paketname) { $Paketname = $Matches[1] }
    }
}
if (-not $Name) { $Name = $Paketname }
$Exe = Join-Path $Zielordner "release\$Name.exe"
if (-not (Test-Path -LiteralPath $Exe)) { Abbruch "Programmdatei fehlt: $Exe" }

# --- 3. Zaehlen -----------------------------------------------------------------------------

$Bytes = [System.IO.File]::ReadAllBytes($Exe)
$Inhalt = $Latin1.GetString($Bytes)

$Pruefungen = @(
    'C:\Users\',
    'C:/Users/',
    $Profil,
    $Profil.Replace('\', '/'),
    $CargoHome,
    $RustupHome,
    $Projekt,
    $Zielordner
)
if ($env:USERNAME) { $Pruefungen += Konto-Muster $env:USERNAME }
$Fehler = 0
Write-Host ''
Write-Host "Zaehlung in $Name.exe ($($Bytes.Length) Bytes), UTF-8 und UTF-16LE, ohne Gross/klein:"
foreach ($p in $Pruefungen) {
    if (-not $p) { continue }
    $n = Zaehle $Inhalt $p $Ohne
    Write-Host ("  {0,6}  {1}" -f $n, (Anzeige $p))
    if ($n -gt 0) { $Fehler++ }
}
if ($env:USERNAME) {
    # Nur zur Auskunft: der Kontoname als Wort, etwa als Name in einem Register.
    $n = Zaehle $Inhalt $env:USERNAME $Ohne
    Write-Host ("  {0,6}  <Konto> als Wort (nur Hinweis, kein Abbruch: Register enthalten Namen)" -f $n)
}
if ($Fehler -gt 0) { Abbruch "Entwicklerpfade oder Kontoname in Pfadform in der Programmdatei ($Fehler Muster)." }

# --- 4. Strenge Importpruefung --------------------------------------------------------------

if (-not $OhneImportpruefung) {
    $env:PROMPTSCAN_EXE = $Exe
    Push-Location -LiteralPath $Projekt
    try {
        & cargo test --locked @JobsArg --test offline importe_der_programmdatei_sind_freigegeben
        if ($LASTEXITCODE -ne 0) { Abbruch "Importpruefung fehlgeschlagen ($LASTEXITCODE)" }
    } finally {
        Pop-Location
        Remove-Item -LiteralPath 'env:PROMPTSCAN_EXE'
    }
}

# --- 5. Pruefsumme --------------------------------------------------------------------------

$Hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $Exe).Hash.ToLowerInvariant()
$Zeile = "$Hash  $Name.exe`n"
[System.IO.File]::WriteAllText("$Exe.sha256", $Zeile, [System.Text.Encoding]::ASCII)
Write-Host ''
Write-Host "SHA-256: $Hash  $Name.exe"
Write-Host "Geschrieben: $(Anzeige "$Exe.sha256")"

# --- 6. Beilagen ----------------------------------------------------------------------------

# Beide sind auch eingebettet (promptscan licenses); als Dateien gehen sie mit den Unterlagen.
$Ausgabe = Split-Path -Parent $Exe
foreach ($b in 'LICENSE.md', 'THIRD-PARTY-NOTICES.md') {
    Copy-Item -LiteralPath (Join-Path $Projekt $b) -Destination (Join-Path $Ausgabe $b) -Force
}
Write-Host "Beigelegt: LICENSE.md, THIRD-PARTY-NOTICES.md in $(Anzeige $Ausgabe)"
exit 0
