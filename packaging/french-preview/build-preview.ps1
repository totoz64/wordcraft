param([Parameter(Mandatory = $true)][ValidatePattern('^[0-9a-f]{40}$')][string] $ExpectedSha)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Set-Location -LiteralPath $root
$head = (& git rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $head -cne $ExpectedSha) { throw 'Checkout HEAD differs from requested source SHA.' }
$short = $head.Substring(0, 8)
$target = 'x86_64-pc-windows-msvc'
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = '-C target-feature=+crt-static'
$env:WORDCRAFT_REQUIRE_WINRES = '1'
$env:WORDCRAFT_BUILD_SHA = $head
$env:WORDCRAFT_BUILD_DATE = [DateTime]::UtcNow.ToString('yyyy-MM-dd')
if (-not $env:CRAFT_FONTS_DIR -or -not $env:CRAFT_I18N_DIR) { throw 'Pinned fonts and interface catalog checkouts must be configured.' }
$env:CRAFT_FONTS_REQUIRED = '1'
$env:CRAFT_I18N_REQUIRED = '1'
& cargo build --release --locked -p wordcraft -p wordcraft-cli --target $target
if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
$dist = Join-Path $root 'dist\french-preview'
$packageName = "wordcraft-preview-$short-windows-x64"
$stage = Join-Path $dist $packageName
if (Test-Path -LiteralPath $dist) { throw 'Preview output directory already exists; use a clean checkout.' }
New-Item -ItemType Directory -Path $stage -Force | Out-Null
$bin = Join-Path $root "target\$target\release"
# Same PE invariants as packaging/windows/package.ps1: real GUI app and console CLI, both x64.
foreach ($entry in @(@('wordcraft.exe', 2), @('wordcraft-cli.exe', 3))) {
    $path = Join-Path $bin $entry[0]
    $bytes = [IO.File]::ReadAllBytes($path)
    if ($bytes.Length -lt 64 -or $bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a) { throw "Not a PE executable: $path" }
    $pe = [BitConverter]::ToInt32($bytes, 0x3c)
    if ($pe -lt 0 -or $pe + 94 -gt $bytes.Length -or [BitConverter]::ToUInt32($bytes, $pe) -ne 0x4550) { throw "Invalid PE header: $path" }
    if ([BitConverter]::ToUInt16($bytes, $pe + 4) -ne 0x8664) { throw "Wrong PE machine: $path" }
    if ([BitConverter]::ToUInt16($bytes, $pe + 0x5c) -ne $entry[1]) { throw "Wrong PE subsystem: $path" }
    Copy-Item -LiteralPath $path -Destination $stage
}
foreach ($name in @('README-francais.txt', 'Lancer-WordCraft.cmd', 'acceptance.json')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination $stage
}
foreach ($name in @('README.md', 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE', 'ATTRIBUTION.md')) {
    Copy-Item -LiteralPath (Join-Path $root $name) -Destination $stage
}
$fontLicences = @(Get-ChildItem -Path (Join-Path $env:CRAFT_FONTS_DIR 'fonts\*\OFL.txt') -File)
if ($fontLicences.Count -eq 0) { throw 'No font OFL notices found.' }
foreach ($ofl in $fontLicences) {
    Copy-Item -LiteralPath $ofl.FullName -Destination (Join-Path $stage "OFL-$($ofl.Directory.Name).txt")
}
$dictionaryRevision = '67032ee957631908098a72edf48dc81cac27e369'
$dictionaryBase = "https://raw.githubusercontent.com/LibreOffice/dictionaries/$dictionaryRevision/fr_FR/dictionaries"
$dictionaryFiles = @(
    @{ name = 'fr.aff'; sha256 = 'c176610cd5dc4846806a65ddd029f422d87978bf58f224aa44222662a16a2de5' },
    @{ name = 'fr.dic'; sha256 = 'b78a868e31dd6e373b6c3217969afb898a9acde828a5e7ef97308da42218c88c' }
)
@{ revision = $dictionaryRevision; base_url = $dictionaryBase; files = $dictionaryFiles; bundled = $false; notice_url = "$dictionaryBase/README_dict_fr.txt" } |
    ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $stage 'dictionary-source.json') -Encoding utf8NoBOM
# CI-only dictionary and profile: never included in the package.
$testRoot = Join-Path $env:RUNNER_TEMP "wordcraft-preview-$short"
if (Test-Path -LiteralPath $testRoot) { throw 'Preview smoke-test directory already exists.' }
$dictionary = Join-Path $testRoot 'dictionaries'
New-Item -ItemType Directory -Force -Path $dictionary | Out-Null
foreach ($file in $dictionaryFiles) {
    $path = Join-Path $dictionary $file.name
    Invoke-WebRequest -Uri "$dictionaryBase/$($file.name)" -OutFile $path
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $file.sha256) { throw "Dictionary hash mismatch: $($file.name)" }
}
$env:WORDCRAFT_DICTIONARIES = $dictionary
$env:WORDCRAFT_PROFILE_DIR = Join-Path $testRoot 'profile'
$env:WORDCRAFT_LANGUAGE = 'fr'
$env:WORDCRAFT_REGION = 'fr-FR'
$env:WORDCRAFT_CONTROL_PORT = $null
$cli = Join-Path $stage 'wordcraft-cli.exe'
$cliVersion = @(& $cli --version)
if ($LASTEXITCODE -ne 0) { throw 'Packaged CLI --version failed.' }
& (Join-Path $PSScriptRoot 'accept-preview.ps1') -Cli $cli -Specification (Join-Path $stage 'acceptance.json') -OutputDirectory (Join-Path $dist 'acceptance-logs')
Copy-Item -LiteralPath (Join-Path $dist 'acceptance-logs\acceptance-result.json') -Destination $stage
$demoDirectory = Join-Path $stage 'demo'
New-Item -ItemType Directory -Path $demoDirectory | Out-Null
$demoText = "Une maisonn près de l’école.`nCCe bateau arrive.`nIl à faim.`nElle a préparer le repas.`nCette boîte ne contient pas grand chose.`n30 centimètres, 20 grammes et 40 watts."
$demoScript = Join-Path $testRoot 'demo.input.json'
@{ commands = @(
    @{ command = 'text.insert'; params = @{ text = $demoText } },
    'select.all',
    @{ command = 'review.language'; params = @{ lang = 'fr-FR'; noProof = $false } }
); save = @((Join-Path $demoDirectory 'essai-francais.docx')) } |
    ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $demoScript -Encoding utf8NoBOM
& $cli run --script $demoScript
if ($LASTEXITCODE -ne 0) { throw 'Could not create original French demonstration DOCX.' }
$binaryHashes = @{}
foreach ($name in @('wordcraft.exe', 'wordcraft-cli.exe')) {
    $binaryHashes[$name] = (Get-FileHash -LiteralPath (Join-Path $stage $name) -Algorithm SHA256).Hash.ToLowerInvariant()
}
$tree = (& git rev-parse 'HEAD^{tree}').Trim()
if ($LASTEXITCODE -ne 0) { throw 'Could not record source tree.' }
$fontSha = (& git -C $env:CRAFT_FONTS_DIR rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $fontSha -cne 'b076417b5880bd4154bd22f00695b1d28c6bea83') { throw 'Unexpected font revision.' }
$i18nSha = (& git -C $env:CRAFT_I18N_DIR rev-parse HEAD).Trim()
$i18nPin = (Get-Content -LiteralPath (Join-Path $root 'craft-i18n.rev') -Raw).Trim()
if ($LASTEXITCODE -ne 0 -or $i18nSha -cne $i18nPin) { throw 'Unexpected i18n revision.' }
$manifest = @{
    protocol = 'wordcraft-french-delivery-build-v1'
    source_sha = $head; source_tree = $tree
    build_date_utc = [DateTime]::UtcNow.ToString('o')
    target = $target; crt = 'static'; signed = $false
    fonts_sha = $fontSha; i18n_sha = $i18nSha
    dictionary_revision = $dictionaryRevision; dictionary_bundled = $false
    binary_sha256 = $binaryHashes; cli_version = $cliVersion
    rustc = @(& rustc --version --verbose); cargo = @(& cargo --version)
    workflow_run = "$env:GITHUB_SERVER_URL/$env:GITHUB_REPOSITORY/actions/runs/$env:GITHUB_RUN_ID"
    cli_acceptance = 'passed_cli_only_manual_gui_pending'
    ui_headless_artifact = "french-delivery-visuals-$head"
    native_gui_acceptance = 'not_run'
    limitations = @('Unsigned experimental fork build, not an official release.', 'Dictionary downloaded locally on first user launch.', 'Other workflow gates and separate headless images require exact-SHA verification.')
}
$manifest | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath (Join-Path $stage 'build-manifest.json') -Encoding utf8NoBOM
# Final package guard: no external dictionary, corpus, profile or history can slip into the ZIP.
$unexpected = @(Get-ChildItem -LiteralPath $stage -Recurse -File | Where-Object { $_.Extension -in @('.aff', '.dic', '.jsonl') })
if ($unexpected.Count -ne 0) { throw 'Unexpected dictionary/corpus payload in preview package.' }
$zip = Join-Path $dist "$packageName-portable.zip"
Compress-Archive -LiteralPath $stage -DestinationPath $zip
$zipHash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
"$zipHash  $packageName-portable.zip" | Set-Content -LiteralPath (Join-Path $dist 'SHA256SUMS.txt') -Encoding ascii
Get-Item -LiteralPath $zip
