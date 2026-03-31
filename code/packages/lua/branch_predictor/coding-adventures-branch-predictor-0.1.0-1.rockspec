package = "coding-adventures-branch-predictor"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Branch prediction algorithms: static, 1-bit, 2-bit saturating counter, BTB, BTFNT",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.branch_predictor"]             = "src/coding_adventures/branch_predictor/init.lua",
        ["coding_adventures.branch_predictor.stats"]       = "src/coding_adventures/branch_predictor/stats.lua",
        ["coding_adventures.branch_predictor.prediction"]  = "src/coding_adventures/branch_predictor/prediction.lua",
        ["coding_adventures.branch_predictor.static"]      = "src/coding_adventures/branch_predictor/static.lua",
        ["coding_adventures.branch_predictor.one_bit"]     = "src/coding_adventures/branch_predictor/one_bit.lua",
        ["coding_adventures.branch_predictor.two_bit"]     = "src/coding_adventures/branch_predictor/two_bit.lua",
        ["coding_adventures.branch_predictor.btb"]         = "src/coding_adventures/branch_predictor/btb.lua",
    },
}
