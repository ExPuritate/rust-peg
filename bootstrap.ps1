# Translated from bootstrap.sh
# Have not tested it yet

function Invoke-NativeCommand() {
    # A handy way to run a command, and automatically throw an error if the
    # exit code is non-zero.

    if ($args.Count -eq 0) {
        throw "Must supply some arguments."
    }

    $command = $args[0]
    $commandArgs = @()
    if ($args.Count -gt 1) {
        $commandArgs = $args[1..($args.Count - 1)]
    }

    $output = (& $command $commandArgs) | Out-String
    $result = $LASTEXITCODE

    if ($result -ne 0) {
        throw "$command $commandArgs exited with code $result."
    }
    return $output
}

$output = Invoke-NativeCommand "cargo" "run" "-p" "peg-macros" "--" "peg-macros/grammar.rustpeg"
Out-File -FilePath "peg-macros/grammar_new.rs" -InputObject $output

Move-Item -Path "peg-macros/grammar.rs" -Destination "peg-macros/grammar_old.rs"
Copy-Item -Path "peg-macros/grammar_new.rs" -Destination "peg-macros/grammar.rs"

cargo run -p "peg-macros" -- "peg-macros/grammar.rustpeg" > peg-macros/grammar_new.rs

if ($LASTEXITCODE -eq 0) {
    Compare-Object -ReferenceObject (Get-Content -Path peg-macros/grammar.rs) -DifferenceObject (Get-Content -Path peg-macros/grammar_new.rs)
    
    rustfmt "peg-macros/grammar.rs"
    $remove = Read-Host "Want to remove old grammar? [Y/n]"
    if ($remove -eq "Y") {
        Remove-Item -Path "peg-macros/grammar_old.rs"
        Remove-Item -Path "peg-macros/grammar_new.rs"
    }
} else {
    Write-Output "Failed"
}