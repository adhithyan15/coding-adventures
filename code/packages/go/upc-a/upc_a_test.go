package upca

import "testing"

func TestUPCA(t *testing.T) {
	checkDigit, err := ComputeUPCACheckDigit("03600029145")
	if err != nil {
		t.Fatal(err)
	}
	if checkDigit != "2" {
		t.Fatalf("unexpected check digit %s", checkDigit)
	}
	runs, err := ExpandUPCARuns("036000291452")
	if err != nil {
		t.Fatal(err)
	}
	totalModules := 0
	for _, run := range runs {
		totalModules += run.Modules
	}
	if totalModules != 95 {
		t.Fatalf("unexpected module count %d", totalModules)
	}
}
