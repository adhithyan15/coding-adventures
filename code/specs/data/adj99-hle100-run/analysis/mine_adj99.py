import json, glob
from collections import defaultdict
FORMAL={'Math','Physics','Engineering','Computer Science/AI','Chemistry'}
items=[]
for f in sorted(glob.glob('code/specs/data/adj99-hle100-run/batches/batch_*.json')):
    if 'partial' in f: continue
    for it in json.load(open(f))['result']['items']:
        items.append(it)
print('total items:', len(items))

def nonerr(d): return d and d.get('flaw_desc')!='[agent-error]'

# 1) fw-haiku error decomposition: reasoning-only vs CAS-extraction
dec=defaultdict(int); dec_by=defaultdict(lambda: defaultdict(int))
for it in items:
    au=it.get('audit_fw_haiku',{})
    sm=au.get('same_model_haiku',{}); cm=au.get('cross_model_opus',{})
    auds=[a for a in (sm,cm) if nonerr(a)]
    grp='FORMAL' if it['category'] in FORMAL else 'INFORMAL'
    if not auds: dec['audit_failed']+=1; continue
    found=any(a.get('found_flaw') for a in auds)
    is_cas=any(a.get('flaw_is_cas_extraction') for a in auds)
    cls = 'no_flaw' if not found else ('cas_extraction' if is_cas else 'reasoning_only')
    dec[cls]+=1; dec_by[grp][cls]+=1
print('\n[1] fw-haiku flaw decomposition (n=%d):'%len(items))
for k in ['reasoning_only','cas_extraction','no_flaw','audit_failed']:
    print(f'   {k:16} {dec[k]}')
print('   by group:')
for g in ['FORMAL','INFORMAL']:
    tot=sum(dec_by[g].values())
    print(f'     {g:8} (n={tot}): reasoning_only={dec_by[g]["reasoning_only"]} cas={dec_by[g]["cas_extraction"]} no_flaw={dec_by[g]["no_flaw"]}')

# 2) provenance_complete x accuracy/defensibility (fw arms)
for arm in ['fw-haiku','fw-opus']:
    pc={'correct':0,'wrong':0}; npc={'correct':0,'wrong':0}; defs_pc=[]; defs_npc=[]
    for it in items:
        a=it['arms'][arm]
        if a['answer']=='[agent-error]': continue
        ok = a['accuracy']=='correct'
        if a.get('provenance_complete'):
            pc['correct' if ok else 'wrong']+=1; defs_pc.append(a['defensibility'])
        else:
            npc['correct' if ok else 'wrong']+=1; defs_npc.append(a['defensibility'])
    m=lambda v: round(sum(v)/len(v),2) if v else None
    print(f'\n[2] {arm}: provenance_complete -> correct/wrong, mean-def')
    print(f'     prov_complete=YES (n={sum(pc.values())}): correct {pc["correct"]}, wrong {pc["wrong"]}, mean-def {m(defs_pc)}')
    print(f'     prov_complete=NO  (n={sum(npc.values())}): correct {npc["correct"]}, wrong {npc["wrong"]}, mean-def {m(defs_npc)}')

# 3) defensibility>=4 but WRONG (confidently grounded but wrong)
for arm in ['plain-opus','fw-opus','fw-haiku']:
    hi=[it for it in items if it['arms'][arm]['answer']!='[agent-error]' and it['arms'][arm]['defensibility']>=4]
    hw=[it for it in hi if it['arms'][arm]['accuracy']!='correct']
    print(f'\n[3] {arm}: def>=4 = {len(hi)}, of which WRONG (incl partial) = {len(hw)} ({round(100*len(hw)/len(hi))}%)')

# 4) fw-opus vs plain-opus item-level accuracy (does retrieval swap items?)
both=neither=fw_only=plain_only=0
for it in items:
    fo=it['arms']['fw-opus']; po=it['arms']['plain-opus']
    if fo['answer']=='[agent-error]' or po['answer']=='[agent-error]': continue
    f=fo['accuracy']=='correct'; p=po['accuracy']=='correct'
    if f and p: both+=1
    elif f and not p: fw_only+=1
    elif p and not f: plain_only+=1
    else: neither+=1
print(f'\n[4] fw-opus vs plain-opus accuracy overlap: both={both}, fw-opus-only={fw_only}, plain-opus-only={plain_only}, neither={neither}')

# 5) accuracy by FORMAL/INFORMAL for the 4 arms
print('\n[5] accuracy (correct) by group:')
for g in ['FORMAL','INFORMAL']:
    sub=[it for it in items if (it['category'] in FORMAL)==(g=='FORMAL')]
    row={}
    for arm in ['plain-haiku','plain-opus','fw-haiku','fw-opus']:
        c=sum(1 for it in sub if it['arms'][arm]['answer']!='[agent-error]' and it['arms'][arm]['accuracy']=='correct')
        row[arm]=c
    print(f'   {g:8} (n={len(sub)}): plain-h {row["plain-haiku"]}, plain-o {row["plain-opus"]}, fw-h {row["fw-haiku"]}, fw-o {row["fw-opus"]}')

# 6) auditor agreement on found_flaw
agree=disagree=0
for it in items:
    au=it.get('audit_fw_haiku',{}); sm=au.get('same_model_haiku',{}); cm=au.get('cross_model_opus',{})
    if not (nonerr(sm) and nonerr(cm)): continue
    if bool(sm.get('found_flaw'))==bool(cm.get('found_flaw')): agree+=1
    else: disagree+=1
print(f'\n[6] auditor agreement on found_flaw (both non-errored, n={agree+disagree}): agree={agree}, disagree={disagree}')
