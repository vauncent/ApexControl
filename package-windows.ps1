$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    $output = Join-Path $PSScriptRoot 'dist'
    $licenses = Join-Path $output 'licenses'
    New-Item -ItemType Directory -Force -Path $licenses | Out-Null
    $executable = Join-Path $output 'ApexControl.exe'
    if (!(Test-Path -LiteralPath $executable) -or
        (Get-FileHash -LiteralPath $executable).Hash -ne (Get-FileHash -LiteralPath 'target\release\apexcontrol.exe').Hash) {
        Copy-Item -LiteralPath 'target\release\apexcontrol.exe' -Destination $executable
    }
    Copy-Item -LiteralPath 'README.md','LICENSE','SECURITY.md','THIRD_PARTY_NOTICES.md','CHANGELOG.md','CHANGELOG.de.md' -Destination $output
    $brandAssets = Join-Path $output 'assets'
    New-Item -ItemType Directory -Force -Path $brandAssets | Out-Null
    Copy-Item -LiteralPath 'assets\apexcontrol.svg','assets\apexcontrol.png','assets\apexcontrol.ico' -Destination $brandAssets
    Copy-Item -LiteralPath '.tools\w64devkit\COPYING.MinGW-w64-runtime.txt' -Destination $licenses
    $metadata = Get-Content -Raw -LiteralPath 'local-data\cargo-metadata.json' | ConvertFrom-Json
    $resolved = @{}
    foreach ($node in $metadata.resolve.nodes) { $resolved[$node.id] = $true }
    $index = [System.Collections.Generic.List[object]]::new()
    foreach ($package in $metadata.packages) {
        if (!$package.source -or !$resolved.ContainsKey($package.id)) { continue }
        $sourceRoot = Split-Path -Parent $package.manifest_path
        $packageFolder = Join-Path $licenses ($package.name + '-' + $package.version)
        New-Item -ItemType Directory -Force -Path $packageFolder | Out-Null
        $files = @(Get-ChildItem -LiteralPath $sourceRoot -Recurse -File | Where-Object {
            $_.Name -match '(LICENSE|LICENCE|COPYING|NOTICE|copyright)|^(OFL|UFL)([._-]|$)' -or
            ($package.name -eq 'epaint_default_fonts' -and $_.Extension -eq '.txt')
        })
        if ($package.license_file) {
            $explicit = Join-Path $sourceRoot $package.license_file
            if (Test-Path -LiteralPath $explicit -PathType Leaf) { $files += Get-Item -LiteralPath $explicit }
        }
        foreach ($file in $files | Sort-Object FullName -Unique) {
            $relative = [IO.Path]::GetRelativePath($sourceRoot, $file.FullName)
            $destination = Join-Path $packageFolder $relative
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
            Copy-Item -LiteralPath $file.FullName -Destination $destination
        }
        $index.Add(@{ name=$package.name; version=$package.version; license=$package.license; repository=$package.repository; authors=$package.authors })
    }
    $index | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $licenses 'packages.json') -Encoding utf8
    Get-FileHash -LiteralPath (Join-Path $output 'ApexControl.exe') -Algorithm SHA256 | ForEach-Object {
        ($_.Hash.ToLowerInvariant() + '  ApexControl.exe') | Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS.txt') -Encoding ascii
    }
    Write-Output ('Packaged ' + $index.Count + ' dependency notices in ' + $output)
} finally { Pop-Location }
