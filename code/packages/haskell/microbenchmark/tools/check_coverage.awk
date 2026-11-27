BEGIN {
    count = split(expected, names, ",")
    for (i = 1; i <= count; i++) required[names[i]] = 1
}
/^-----<module / {
    module = $0
    sub(/^.*\//, "", module)
    sub(/>.*/, "", module)
}
/expressions used \(/ {
    if (module in required) {
        counts = $0
        sub(/^.*\(/, "", counts)
        sub(/\).*/, "", counts)
        split(counts, values, "/")
        hit[module] = values[1] + 0
        total[module] = values[2] + 0
        measured[module] = 1
        if (total[module] == 0 || 100 * hit[module] / total[module] <= 80) {
            printf "%s expression coverage must exceed 80%% with a nonzero denominator; got %d/%d\n", module, hit[module], total[module] > "/dev/stderr"
            failed = 1
        }
        printf "%s expression coverage: %.2f%% (%d/%d)\n", module, 100 * hit[module] / total[module], hit[module], total[module]
    }
}
END {
    for (name in required) {
        if (!measured[name]) {
            printf "no measurable HPC module found for %s\n", name > "/dev/stderr"
            failed = 1
        }
    }
    if (failed) exit 1
}
