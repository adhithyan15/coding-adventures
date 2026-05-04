(cd ../../../packages/python/single-layer-network && python3 -m pytest tests)
(cd ../../../packages/python/two-layer-network && python3 -m pytest tests)
(cd ../../../packages/python/neural-network && python3 -m pytest tests)
(cd ../../../packages/python/neural-graph-vm && PYTHONPATH=../neural-network/src:src python3 -m pytest tests)
PYTHONPATH=../../../packages/python/two-layer-network/src:../../../packages/python/single-layer-network/src:../../../packages/python/neural-network/src:../../../packages/python/neural-graph-vm/src python3 main.py
