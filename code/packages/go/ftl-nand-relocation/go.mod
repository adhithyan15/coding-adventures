module github.com/adhithyan15/coding-adventures/code/packages/go/ftl-nand-relocation

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/ftl-erase-role-evidence v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/ftl-erase-victim v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/ftl-evacuation-plan v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts v0.0.0
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map => ../ftl-page-map

replace github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts => ../nand-page-program-cuts

replace github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts => ../nand-block-erase-cuts

replace github.com/adhithyan15/coding-adventures/code/packages/go/ftl-erase-victim => ../ftl-erase-victim

replace github.com/adhithyan15/coding-adventures/code/packages/go/ftl-evacuation-plan => ../ftl-evacuation-plan

replace github.com/adhithyan15/coding-adventures/code/packages/go/ftl-erase-role-evidence => ../ftl-erase-role-evidence
