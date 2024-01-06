function ThrowOnNativeFailure {
    if (-not $?)
    {
        throw 'Native Failure'
    }
}

cargo test
ThrowOnNativeFailure

wsl cargo test
ThrowOnNativeFailure
