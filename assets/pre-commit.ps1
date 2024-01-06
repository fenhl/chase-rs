function ThrowOnNativeFailure {
    if (-not $?)
    {
        throw 'Native Failure'
    }
}

cargo check --all-features
ThrowOnNativeFailure

cargo test
ThrowOnNativeFailure

wsl cargo check --all-features
ThrowOnNativeFailure

wsl cargo test
ThrowOnNativeFailure
