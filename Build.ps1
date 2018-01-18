# This script originally (c) 2016 Serilog Contributors - license Apache 2.0
$branch = @{ $true = $env:APPVEYOR_REPO_BRANCH; $false = $(git symbolic-ref --short -q HEAD) }[$env:APPVEYOR_REPO_BRANCH -ne $NULL];
$revision = @{ $true = ""; $false = "-local" }[$env:APPVEYOR_BUILD_NUMBER -ne $NULL];
$suffix = @{ $true = ""; $false = "$($branch.Substring(0, [math]::Min(10,$branch.Length)))$revision"}[$branch -eq "master" -and $revision -ne "local"]

$version = "1.0."
if ($env:APPVEYOR_BUILD_NUMBER -ne $NULL) {
    $version = $version + $env:APPVEYOR_BUILD_NUMBER
} else {
    $version = $version + "0"
}

if ($suffix) {
    $version = $version + "-" + $suffix
}

if (Test-Path .\publish) {
    Remove-Item .\publish -Recurse
}

mkdir .\publish

cargo build --release 2>&1
if ($LASTEXITCODE) { exit 1 }

& .\tool\nuget.exe pack .\Seq.App.JsonArchive.nuspec -version $version -outputdirectory .\publish
exit $LASTEXITCODE
