(cd ../../../packages/python/single-layer-network && python3 -m pytest tests)
(cd ../../../packages/python/two-layer-network && python3 -m pytest tests)
(cd ../../../packages/python/multi-directed-graph && python3 -m pytest tests)
(cd ../../../packages/python/neural-network && PYTHONPATH=src:../multi-directed-graph/src python3 -m pytest tests)
(cd ../../../packages/python/neural-graph-vm && PYTHONPATH=../multi-directed-graph/src:../neural-network/src:src python3 -m pytest tests)
PYTHONPATH=../../../packages/python/two-layer-network/src:../../../packages/python/single-layer-network/src:../../../packages/python/multi-directed-graph/src:../../../packages/python/neural-network/src:../../../packages/python/neural-graph-vm/src python3 main.py
