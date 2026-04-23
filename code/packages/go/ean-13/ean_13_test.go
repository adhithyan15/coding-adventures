package ean13

import "testing"

func TestEAN13(t *testing.T) {
	checkDigit, err := ComputeEAN13CheckDigit("400638133393")
	if err != nil {
		t.Fatal(err)
	}
	if checkDigit != "1" {
		t.Fatalf("unexpected check digit %s", checkDigit)
	}
	parity, err := LeftParityPattern("4006381333931")
	if err != nil {
		t.Fatal(err)
	}
	if parity != "LGLLGG" {
		t.Fatalf("unexpected parity %s", parity)
	}
	runs, err := ExpandEAN13Runs("4006381333931")
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
