#requires -Version 7.4
# Structural validation only. Cargo tests apply Rust semantic validation to
# the same cases, including cases that intentionally pass this Schema.
param([switch]$AuditRaw)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$schemaPath = Join-Path $repoRoot 'docs/schemas/sky-score-v1.schema.json'
$exampleJson = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'docs/examples/sky-15-example.json')
$fixturesDir = Join-Path $repoRoot 'crates/skysheet-score/tests/fixtures'

function Get-FixtureParent($Value, [string]$Pointer) {
    # Fixtures use only plain object keys and array indices. This is not a
    # general JSON Patch implementation.
    $parts = $Pointer.TrimStart('/').Split('/')
    $parent = $Value
    for ($index = 0; $index -lt $parts.Length - 1; $index++) {
        if ($parent -is [System.Collections.IDictionary]) {
            if (-not $parent.Contains($parts[$index])) { throw "Unknown fixture path: $Pointer" }
            $parent = $parent[$parts[$index]]
        } else {
            $parent = $parent[[int]$parts[$index]]
        }
    }
    if ($parent -isnot [System.Collections.IDictionary] -and $parent -isnot [System.Collections.IList]) {
        throw "Expected object or array parent: $Pointer"
    }
    return ,$parent
}

if (-not (Test-Json -Json $exampleJson -SchemaFile $schemaPath)) {
    throw 'The documented example does not satisfy the Schema.'
}
$total = 0
foreach ($fixtureName in @('contract-boundaries.json', 'contract-conformance.json', 'object-shapes.json')) {
    $cases = Get-Content -Raw -LiteralPath (Join-Path $fixturesDir $fixtureName) | ConvertFrom-Json -AsHashtable
    if ($cases.Count -eq 0) { throw "No cases found in $fixtureName" }
    foreach ($case in $cases) {
        $value = ConvertFrom-Json -InputObject $exampleJson -AsHashtable
        foreach ($pointer in $case.removes) {
            $parent = Get-FixtureParent $value $pointer
            $key = $pointer.Split('/')[-1]
            if (-not $parent.Contains($key)) { throw "Unknown fixture removal: $pointer" }
            $parent.Remove($key)
        }
        foreach ($pointer in $case.changes.Keys) {
            if ($pointer -eq '') {
                $value = $case.changes[$pointer]
            } else {
                $parent = Get-FixtureParent $value $pointer
                $key = $pointer.Split('/')[-1]
                if ($parent -is [System.Collections.IDictionary]) { $parent[$key] = $case.changes[$pointer] }
                else { $parent[[int]$key] = $case.changes[$pointer] }
            }
        }
        $json = ConvertTo-Json -InputObject $value -Depth 30
        $valid = Test-Json -Json $json -SchemaFile $schemaPath -ErrorAction SilentlyContinue
        if ($valid -ne $case.schema_valid) {
            throw "Schema expectation failed: $($case.name); expected $($case.schema_valid), got $valid"
        }
        $total++
    }
}
Write-Output "Schema validation passed: documented example and $total shared cases."

if ($AuditRaw) {
    # Validator observations only, not full score validity. The direct Rust
    # expectations are now asserted by normal Cargo tests after D2-D4 approval.
    # Numeric/member probes retain original text; shape probes use replacements.
    $probes = Get-Content -Raw -LiteralPath (Join-Path $fixturesDir 'raw-input-audit.json') | ConvertFrom-Json -AsHashtable
    foreach ($probe in $probes) {
        if ($probe.Contains('find')) {
            $index = $exampleJson.IndexOf($probe.find, [StringComparison]::Ordinal)
            if ($index -lt 0) { throw "Missing audit token: $($probe.name)" }
            $raw = $exampleJson.Remove($index, $probe.find.Length).Insert($index, $probe.replace)
        } else {
            # Object-shape probes do not depend on original numeric spelling.
            $value = ConvertFrom-Json -InputObject $exampleJson -AsHashtable
            foreach ($pointer in $probe.changes.Keys) {
                $parts = $pointer.TrimStart('/').Split('/')
                $parent = $value
                for ($index = 0; $index -lt $parts.Length - 1; $index++) {
                    if ($parent -is [System.Collections.IDictionary]) { $parent = $parent[$parts[$index]] }
                    else { $parent = $parent[[int]$parts[$index]] }
                }
                if ($parent -is [System.Collections.IDictionary]) { $parent[$parts[-1]] = $probe.changes[$pointer] }
                else { $parent[[int]$parts[-1]] = $probe.changes[$pointer] }
            }
            $raw = ConvertTo-Json -InputObject $value -Depth 30
        }
        $rawErrors = @()
        $valid = Test-Json -Json $raw -SchemaFile $schemaPath -ErrorAction SilentlyContinue -ErrorVariable rawErrors
        [ordered]@{
            name = $probe.name
            schema_valid = $valid
            diagnostics = @($rawErrors | ForEach-Object { $_.ToString() })
        } | ConvertTo-Json -Compress
    }
}
