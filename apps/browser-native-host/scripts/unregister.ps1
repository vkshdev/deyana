param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Chrome', 'Edge')]
    [string]$Browser
)

$vendor = if ($Browser -eq 'Chrome') { 'Google\Chrome' } else { 'Microsoft\Edge' }
$registryPath = "HKCU:\Software\$vendor\NativeMessagingHosts\app.deyana.browser"
if (Test-Path -LiteralPath $registryPath) {
    Remove-Item -LiteralPath $registryPath -Recurse -Force
}

Write-Output "Unregistered app.deyana.browser for $Browser."
