go vet ./...
go test -race ./... -v -cover
go build ./...
go test -run=^$ -fuzz=FuzzShortestPathWeightPreconditions -fuzztime=5s -parallel=1 .
go test -run=^$ -fuzz=FuzzConcurrentGraphModelHistories -fuzztime=5s -parallel=1 .
go test -run=^$ -fuzz=FuzzEdgeIdentityModel -fuzztime=5s -parallel=1 .
go test -run=^$ -fuzz=FuzzShortestPathDistanceOverflow -fuzztime=5s -parallel=1 .
go test -run=^$ -fuzz=FuzzCheckedEdgeMutationClasses -fuzztime=5s -parallel=1 .
