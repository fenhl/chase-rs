function ThrowOnNativeFailure {
    if (-not $?)
    {
        throw 'Native Failure'
    }
}

cargo check
ThrowOnNativeFailure

wsl cargo check
ThrowOnNativeFailure
