go vet ./...
go test ./... -race -cover
go test -fuzz=FuzzAnalyzeSinglePair -fuzztime=5s .
go test -fuzz=FuzzAnalyzePairSetInvariants -fuzztime=5s .
go test -fuzz=FuzzPerformanceTicksNS -fuzztime=5s .
go build ./...
python3 ../../../../code/scripts/microbenchmark-installed-consumer.py go
