import json, glob, statistics as st
arms=['plain-haiku','plain-opus','fw-haiku','fw-opus']
defn={k:[] for k in arms}; acc={k:{'correct':0,'partial':0,'incorrect':0} for k in arms}; tot={k:0 for k in arms}
errs={k:0 for k in arms}
hi={k:0 for k in arms}  # defensibility>=4
fw_n=0; same_flaw=0; cross_flaw=0; either_flaw=0; same_err=0; cross_err=0
cas_flag=0; flagged_facts=0
by_cat={}
prov_complete=0
for f in sorted(glob.glob('code/specs/data/adj99-hle100-run/batches/batch_*.json')):
    if 'partial' in f: continue
    for it in json.load(open(f))['result']['items']:
        cat=it.get('category','?'); by_cat.setdefault(cat,{k:[] for k in arms})
        for k in arms:
            a=it['arms'][k]
            if a['answer']=='[agent-error]':
                errs[k]+=1; continue
            defn[k].append(a['defensibility']); tot[k]+=1
            if a['defensibility']>=4: hi[k]+=1
            g=a['accuracy']; acc[k][g]=acc[k].get(g,0)+1
            by_cat[cat][k].append(a['defensibility'])
        fw_n+=1
        au=it.get('audit_fw_haiku',{})
        sm=au.get('same_model_haiku',{}); cm=au.get('cross_model_opus',{})
        sm_err = sm.get('flaw_desc')=='[agent-error]'
        cm_err = cm.get('flaw_desc')=='[agent-error]'
        if sm_err: same_err+=1
        elif sm.get('found_flaw'): same_flaw+=1
        if cm_err: cross_err+=1
        elif cm.get('found_flaw'): cross_flaw+=1
        sm_f=sm.get('found_flaw') and not sm_err; cm_f=cm.get('found_flaw') and not cm_err
        if sm_f or cm_f: either_flaw+=1
        sm_cas=sm.get('flaw_is_cas_extraction') and not sm_err
        cm_cas=cm.get('flaw_is_cas_extraction') and not cm_err
        if sm_cas or cm_cas: cas_flag+=1
        flagged_facts += len(set((sm.get('flagged_cas_facts') or []) if not sm_err else []) | set((cm.get('flagged_cas_facts') or []) if not cm_err else []))

mean=lambda v: round(sum(v)/len(v),3) if v else None
agg={
 'n_items':fw_n,
 'arms':{k:{'mean_defensibility':mean(defn[k]),'n_scored':tot[k],'def_ge4':hi[k],
            'accuracy':acc[k],'agent_errors':errs[k]} for k in arms},
 'audit_trail':{'fw_haiku_items':fw_n,
   'same_model_haiku_found_flaw':same_flaw,'same_model_agent_errors':same_err,
   'cross_model_opus_found_flaw':cross_flaw,'cross_model_agent_errors':cross_err,
   'either_found_flaw':either_flaw,
   'flaw_is_cas_extraction':cas_flag,'distinct_flagged_cas_facts':flagged_facts},
 'total_arm_agent_errors':sum(errs.values()),
 'by_category_mean_defensibility':{c:{k:mean(by_cat[c][k]) for k in arms} for c in sorted(by_cat)},
}
json.dump(agg, open('code/specs/data/adj99-hle100-run/aggregate.json','w'), indent=2)
print(json.dumps({k:agg[k] for k in ['n_items','arms','audit_trail','total_arm_agent_errors']}, indent=2))
