package = "coding-adventures-test-bootstrap"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Property-based testing engine for Lua (VER05o): SplitMix64, integrated shrinking, honest verdicts, pbt-v1 conformance",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.test_bootstrap"] = "src/coding_adventures/test_bootstrap/init.lua",
        ["coding_adventures.test_bootstrap.budget"] = "src/coding_adventures/test_bootstrap/budget.lua",
        ["coding_adventures.test_bootstrap.canon"] = "src/coding_adventures/test_bootstrap/canon.lua",
        ["coding_adventures.test_bootstrap.corpus"] = "src/coding_adventures/test_bootstrap/corpus.lua",
        ["coding_adventures.test_bootstrap.failure_ids"] = "src/coding_adventures/test_bootstrap/failure_ids.lua",
        ["coding_adventures.test_bootstrap.gen"] = "src/coding_adventures/test_bootstrap/gen.lua",
        ["coding_adventures.test_bootstrap.gens"] = "src/coding_adventures/test_bootstrap/gens.lua",
        ["coding_adventures.test_bootstrap.hooks"] = "src/coding_adventures/test_bootstrap/hooks.lua",
        ["coding_adventures.test_bootstrap.int"] = "src/coding_adventures/test_bootstrap/int.lua",
        ["coding_adventures.test_bootstrap.limits"] = "src/coding_adventures/test_bootstrap/limits.lua",
        ["coding_adventures.test_bootstrap.prng"] = "src/coding_adventures/test_bootstrap/prng.lua",
        ["coding_adventures.test_bootstrap.runner"] = "src/coding_adventures/test_bootstrap/runner.lua",
        ["coding_adventures.test_bootstrap.safe"] = "src/coding_adventures/test_bootstrap/safe.lua",
        ["coding_adventures.test_bootstrap.shrink"] = "src/coding_adventures/test_bootstrap/shrink.lua",
        ["coding_adventures.test_bootstrap.strict_json"] = "src/coding_adventures/test_bootstrap/strict_json.lua",
        ["coding_adventures.test_bootstrap.tfv"] = "src/coding_adventures/test_bootstrap/tfv.lua",
        ["coding_adventures.test_bootstrap.values"] = "src/coding_adventures/test_bootstrap/values.lua",
    },
}
