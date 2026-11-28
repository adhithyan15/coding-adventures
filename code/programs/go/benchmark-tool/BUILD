go build -o benchmark-tool .
go test ./... -v -cover
go test -run=^$ -fuzz=FuzzPairedReportIngress -fuzztime=5s .
go test -run=^$ -fuzz=FuzzSummaryImport -fuzztime=5s .
go test -run=^$ -fuzz=FuzzTrialRecordIngress -fuzztime=5s .

go vet ./...
