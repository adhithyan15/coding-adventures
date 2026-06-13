# /// script
# requires-python = ">=3.11"
# dependencies = ["pyarrow"]
# ///
import pyarrow.parquet as pq, json, random
pqf='/Users/adhithya/.cache/huggingface/hub/datasets--cais--hle/snapshots/5a81a4c7271a2a2a312b9a690f0c2fde837e4c29/data/test-00000-of-00001.parquet'
t=pq.read_table(pqf).to_pylist()
ex=set(json.load(open('/tmp/exclude_ids.json')))
def textonly(r): return not (r.get('image') or r.get('image_preview'))
cands=[r for r in t if textonly(r) and r.get('answer_type')=='exactMatch' and r.get('category') and str(r['id'])[:10] not in ex]
from collections import defaultdict
strata=defaultdict(list)
for r in cands: strata[r['category']].append(r)
print('strata:', {k:len(v) for k,v in sorted(strata.items())})
rng=random.Random(99)
for v in strata.values(): rng.shuffle(v)
cats=sorted(strata, key=lambda k:-len(strata[k]))
picks=[]; ci=0
while len(picks)<100:
    c=cats[ci%len(cats)]
    if strata[c]: picks.append(strata[c].pop())
    ci+=1
out=[{'id':str(r['id'])[:10],'question':r['question'],'answer':str(r['answer']),'category':r['category']} for r in picks]
json.dump(out, open('code/specs/data/adj99-hle100-run/items_100.json','w'), ensure_ascii=False, indent=0)
from collections import Counter
print('sampled', len(out), 'items; by category:', dict(Counter(o['category'] for o in out)))
