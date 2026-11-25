#!/bin/sh
set -eu

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
package=$(CDPATH= cd "$script_dir/.." && pwd)
distribution="$package/gradle-build/install/microbenchmark"
temp=$(mktemp -d "${TMPDIR:-/tmp}/java-microbenchmark-consumer.XXXXXX")
trap 'rm -rf "$temp"' EXIT HUP INT TERM

classpath="$distribution/lib/*"
javac -classpath "$classpath" -d "$temp" "$package/tests/installed-consumer/Consumer.java"
java -classpath "$temp:$classpath" Consumer

driver_class=com.codingadventures.microbenchmark.ArrayMicrobenchmark
report="$temp/native-report.json"
java -classpath "$classpath" "$driver_class" --pairs 40 --revision installed-consumer >"$report"
java -classpath "$classpath" "$driver_class" --replay "$report"
