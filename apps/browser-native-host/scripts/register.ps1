param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[a-p]{32}$')]
    [string]$ExtensionId,

    [Parameter(Mandatory = $true)]
    [ValidateSet('Chrome', 'Edge')]
    [string]$Browser,

    [string]$HostBinary = (Join-Path $PSScriptRoot '..\target\release\deyana-browser-native-host.exe')
)

$resolvedBinary = (Resolve-Path -LiteralPath $HostBinary -ErrorAction Stop).Path
$hostDirectory = Split-Path -Parent $resolvedBinary
$origin = "chrome-extension://$ExtensionId/"
$manifestPath = Join-Path $hostDirectory "app.deyana.browser.$($Browser.ToLowerInvariant()).json"
$originsPath = Join-Path $hostDirectory 'browser-native-origins.json'

$manifest = [ordered]@{
    name = 'app.deyana.browser'
    description = 'Deyana local browser bridge'
    path = $resolvedBinary
    type = 'stdio'
    allowed_origins = @($origin)
}
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$manifestJson = $manifest | ConvertTo-Json -Depth 4
[System.IO.File]::WriteAllText($manifestPath, $manifestJson, $utf8NoBom)

$existingOrigins = @()
if (Test-Path -LiteralPath $originsPath) {
    $existingOrigins = @((Get-Content -Raw -LiteralPath $originsPath | ConvertFrom-Json).allowedOrigins)
}
$allOrigins = @($existingOrigins + $origin | Sort-Object -Unique)
$originsJson = [ordered]@{ allowedOrigins = $allOrigins } | ConvertTo-Json -Depth 3
[System.IO.File]::WriteAllText($originsPath, $originsJson, $utf8NoBom)

$vendor = if ($Browser -eq 'Chrome') { 'Google\Chrome' } else { 'Microsoft\Edge' }
$registryPath = "HKCU:\Software\$vendor\NativeMessagingHosts\app.deyana.browser"
New-Item -Path $registryPath -Force | Out-Null
Set-Item -Path $registryPath -Value $manifestPath

Write-Output "Registered app.deyana.browser for $Browser and extension $ExtensionId."
