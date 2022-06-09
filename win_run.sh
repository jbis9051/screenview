set -e

cargo build --target=x86_64-pc-windows-gnu || exit 1

prev=""
if [ -f target/x86_64-pc-windows-gnu/debug/native_test.exe.sha1 ]; then
  prev=$(cat target/x86_64-pc-windows-gnu/debug/native_test.exe.sha1)
fi
new=$(shasum target/x86_64-pc-windows-gnu/debug/native_test.exe)
if [ "$prev" = "$new" ]; then
    echo "no change in native_test.exe"

else
  echo "change detected"
  echo "$new" > target/x86_64-pc-windows-gnu/debug/native_test.exe.sha1
  scp -P 9000 target/x86_64-pc-windows-gnu/debug/native_test.exe josh@127.0.0.1:C:/Users/josh/Desktop/
fi
ssh josh@127.0.0.1 -p 9000 "C:\Users\josh\Desktop\native_test.exe"