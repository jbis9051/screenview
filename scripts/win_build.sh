# Builds the project for Windows and transfers it to a VM via SSH listening on localhost:9000
#
# Should be run from screenview/packages/rust/native_test
# native_test isn't version control, but it's basically just a crate with:
#
# [dependencies]
# native = {path = "../native"}

set -e

opt=""
if [ "$1" = "--release" ]; then
  opt="--release"
fi

cargo build $opt --target=x86_64-pc-windows-gnu || exit 1

prev=""
if [ -f target/x86_64-pc-windows-gnu/$1/native_test.exe.sha1 ]; then
  prev=$(cat target/x86_64-pc-windows-gnu/$1/native_test.exe.sha1)
fi
new=$(shasum target/x86_64-pc-windows-gnu/$1/native_test.exe)
if [ "$prev" = "$new" ]; then
  echo "no change in native_test.exe"
else
  echo "change detected"
  scp -P 9000 target/x86_64-pc-windows-gnu/$1/native_test.exe josh@127.0.0.1:'C:\Users\josh\Desktop\'
  echo "$new" >target/x86_64-pc-windows-gnu/$1/native_test.exe.sha1
fi
echo "build complete"