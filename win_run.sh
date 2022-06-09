set -e

cargo build --target=x86_64-pc-windows-gnu || exit 1
scp -P 9000 target/x86_64-pc-windows-gnu/debug/native_test.exe josh@127.0.0.1:C:/Users/josh/Desktop/ && #ssh -t josh@127.0.0.1 -p 9000 'start cmd.exe /k "cd C:\Users\josh\Desktop\ && native_test.exe"'