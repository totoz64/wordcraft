param(
    [Parameter(Mandatory = $true)][string] $Cli,
    [Parameter(Mandatory = $true)][string] $Specification,
    [Parameter(Mandatory = $true)][string] $OutputDirectory
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$OutputDirectory = (Resolve-Path -LiteralPath $OutputDirectory).Path
$spec = Get-Content -LiteralPath $Specification -Raw -Encoding utf8 | ConvertFrom-Json -Depth 100
function Equal-Ordinal([string] $Left, [string] $Right) {
    return [string]::Equals($Left, $Right, [StringComparison]::Ordinal)
}
function Invoke-Script([string] $Id, [object[]] $Commands, [string[]] $Save = @()) {
    $scriptPath = Join-Path $OutputDirectory "$Id.input.json"
    @{ commands = $Commands; save = $Save } | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $scriptPath -Encoding utf8NoBOM
    $lines = @(& $Cli run --script $scriptPath --print 2> (Join-Path $OutputDirectory "$Id.stderr.txt"))
    if ($LASTEXITCODE -ne 0) { throw "CLI case $Id failed (exit $LASTEXITCODE); see stderr." }
    $lines | Set-Content -LiteralPath (Join-Path $OutputDirectory "$Id.stdout.txt") -Encoding utf8NoBOM
    return $lines
}
function Base-Commands([string] $Text, [string] $Lang) {
    return @(
        @{ command = 'text.insert'; params = @{ text = $Text } },
        'select.all',
        @{ command = 'review.language'; params = @{ lang = $Lang; noProof = $false } },
        @{ command = 'review.proofing'; params = @{ value = $true } }
    )
}
$results = [System.Collections.Generic.List[object]]::new()
$failed = $false
foreach ($case in $spec.cases) {
    $checks = [System.Collections.Generic.List[object]]::new()
    $lines = @(Invoke-Script $case.id ((Base-Commands $case.text $case.lang) + @('review.issues')))
    $issueLines = @($lines | Where-Object { $_.StartsWith('review.issues: ', [StringComparison]::Ordinal) })
    if ($issueLines.Count -ne 1) { throw "Missing/unexpected review.issues output for $($case.id)" }
    $issues = @($issueLines[0].Substring('review.issues: '.Length) | ConvertFrom-Json -Depth 100)
    $e = $case.expect
    if ($e.PSObject.Properties.Name -contains 'issueText') {
        $matching = @($issues | Where-Object { (Equal-Ordinal $_.text $e.issueText) -and (Equal-Ordinal $_.kind $e.kind) })
        $checks.Add(@{ name = 'expected_issue'; passed = ($matching.Count -gt 0) })
        if ($matching.Count -gt 0) {
            $issue = $matching[0]
            $index = $case.text.IndexOf($e.issueText, [StringComparison]::Ordinal)
            $start = [Text.Encoding]::UTF8.GetByteCount($case.text.Substring(0, $index))
            $end = $start + [Text.Encoding]::UTF8.GetByteCount($e.issueText)
            $checks.Add(@{ name = 'original_utf8_range'; passed = ($issue.start.off -eq $start -and $issue.end.off -eq $end); expected = @($start, $end); observed = @($issue.start.off, $issue.end.off) })
            if ($e.PSObject.Properties.Name -contains 'suggestion') {
                $found = @($issue.suggestions | Where-Object { Equal-Ordinal $_ $e.suggestion }).Count -gt 0
                $checks.Add(@{ name = 'expected_suggestion'; passed = $found })
            }
            if ($e.PSObject.Properties.Name -contains 'firstSuggestion') {
                $found = @($issue.suggestions).Count -gt 0 -and (Equal-Ordinal $issue.suggestions[0] $e.firstSuggestion)
                $checks.Add(@{ name = 'first_suggestion'; passed = $found })
            }
        }
    }
    if ($e.PSObject.Properties.Name -contains 'noSpelling') {
        foreach ($word in $e.noSpelling) {
            $bad = @($issues | Where-Object { (Equal-Ordinal $_.kind 'spelling') -and (Equal-Ordinal $_.text $word) })
            $checks.Add(@{ name = "no_spelling:$word"; passed = ($bad.Count -eq 0) })
        }
    }
    if ($e.PSObject.Properties.Name -contains 'noSuggestion') {
        $bad = @($issues | Where-Object { @($_.suggestions | Where-Object { Equal-Ordinal $_ $e.noSuggestion }).Count -gt 0 })
        $checks.Add(@{ name = 'bounded_abstention'; passed = ($bad.Count -eq 0) })
    }
    $passed = @($checks | Where-Object { -not $_.passed }).Count -eq 0
    if (-not $passed) { $failed = $true }
    $results.Add(@{ id = $case.id; text = $case.text; lang = $case.lang; expected = $e; passed = $passed; checks = $checks.ToArray(); observed_issues = $issues })
}
$correction = $spec.correction_undo
$base = Base-Commands $correction.text 'fr-FR'
$apply = @('review.spelling', @{ command = 'review.applySuggestion'; params = @{ text = $correction.replacement } })
$correctedPath = Join-Path $OutputDirectory 'corrected.docx'
$restoredPath = Join-Path $OutputDirectory 'restored.docx'
$null = Invoke-Script 'apply' ($base + $apply) @($correctedPath)
$null = Invoke-Script 'undo' ($base + $apply + @('edit.undo')) @($restoredPath)
$corrected = @(& $Cli text $correctedPath 2> (Join-Path $OutputDirectory 'corrected.stderr.txt'))
if ($LASTEXITCODE -ne 0) { throw 'Could not read corrected document.' }
$restored = @(& $Cli text $restoredPath 2> (Join-Path $OutputDirectory 'restored.stderr.txt'))
if ($LASTEXITCODE -ne 0) { throw 'Could not read restored document.' }
$applied = Equal-Ordinal ($corrected -join "`n") $correction.corrected
$undone = Equal-Ordinal ($restored -join "`n") $correction.text
if (-not ($applied -and $undone)) { $failed = $true }
$report = @{
    protocol = 'wordcraft-french-delivery-acceptance-result-v1'
    specification_protocol = $spec.protocol
    measured_at_utc = [DateTime]::UtcNow.ToString('o')
    source_sha = $env:WORDCRAFT_BUILD_SHA
    cli_sha256 = (Get-FileHash -LiteralPath $Cli -Algorithm SHA256).Hash.ToLowerInvariant()
    status = $(if ($failed) { 'failed' } else { 'passed_cli_only_manual_gui_pending' })
    cases = $results.ToArray()
    correction_undo = @{ applied = $applied; undone = $undone; observed_corrected = $corrected; observed_restored = $restored }
    limitations = @('Small development cases; no global quality score.', 'Native GUI launch/click/render acceptance is still required.', 'No Word data or corpus was used.')
}
$report | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'acceptance-result.json') -Encoding utf8NoBOM
if ($failed) { throw 'Preview CLI acceptance failed; see acceptance-result.json.' }
