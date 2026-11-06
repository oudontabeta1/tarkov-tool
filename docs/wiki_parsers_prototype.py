import re, json

def clean(s):
    s = re.sub(r'<font[^>]*>|</font>', '', s)
    s = re.sub(r'<br\s*/?>', ' / ', s)
    s = re.sub(r"'''?", '', s)
    s = re.sub(r'\[\[File:[^\]|]*\|(?:[^\]|]*\|)*?([^\]|=]+)\]\]', lambda m: '' if re.fullmatch(r'(frameless|\d+px|\d*x?\d+px|center|link)', m.group(1).strip()) else '['+m.group(1).strip()+']', s)
    s = re.sub(r'\[\[File:[^\]]*\]\]', '', s)
    s = re.sub(r'\[\[(?:[^\]|]*\|)?([^\]]*)\]\]', r'\1', s)
    s = re.sub(r'<[^>]+>', '', s)
    return s.strip()

def links(s):
    out = []
    for m in re.finditer(r'\[\[([^\]|#]+)(?:#[^\]|]*)?(?:\|[^\]]*)?\]\]', s):
        t = m.group(1).strip()
        if not t.lower().startswith(('file:', 'category:')):
            out.append(t)
    return out

def infobox(text, name):
    m = re.search(r'\{\{\s*' + re.escape(name) + r'\s*\n(.*?)\n\}\}', text, re.S)
    if not m: return {}
    d = {}
    for line in m.group(1).splitlines():
        mm = re.match(r'\|\s*([^=|]+?)\s*=\s*(.*)$', line)
        if mm: d[mm.group(1).strip()] = mm.group(2).strip()
    return d

def sections(text):
    parts = re.split(r'^==\s*([^=]+?)\s*==\s*$', text, flags=re.M)
    return {parts[i]: parts[i+1] for i in range(1, len(parts), 2)}

def table_rows(text):
    """generic wikitable -> list of rows (list of raw cell strings)"""
    rows = []
    for block in re.split(r'^\|-\s*$', text, flags=re.M):
        cells, cur = [], None
        for line in block.splitlines():
            if line.startswith('|}'):
                cur = None
                break
            if line and line[0] in '|!' and not line.startswith(('{|', '|+')):
                body = line[1:]
                if re.match(r'^\s*(?:[\w-]+="[^"]*"\s*)+\|(?!\|)', body):
                    body = body.split('|', 1)[1]
                cur = [body]; cells.append(cur)
            elif cur is not None:
                cur.append(line)
        if cells:
            rows.append(['\n'.join(c).strip() for c in cells])
    return rows

def num(s):
    m = re.search(r'[-+]?\d[\d,]*\.?\d*', clean(s))
    return float(m.group().replace(',', '')) if m else None

# ---------- quests ----------
def parse_quest(title, text):
    ib = infobox(text, 'Infobox quest')
    sec = sections(text)
    q = {'title': title, 'trader': clean(ib.get('given by', '')),
         'maps': links(ib.get('location', '')),
         'loyalty': num(ib.get('LL requirement', '')),
         'previous_raw': ib.get('previous', ''), 'previous': [l for l in links(ib.get('previous', '')) if l != title],
         'previous_see_requirements': 'See requirements' in ib.get('previous', ''),
         'leads_to': links(ib.get('leads to', '')),
         'kappa': clean(ib.get('reqkappa', '')) or None}
    # requirements section
    req = sec.get('Requirements', '')
    m = re.search(r'level (\d+)', req, re.I)
    q['player_level'] = int(m.group(1)) if m else None
    q['req_quests'] = [l for ln in req.splitlines() if ln.startswith('**') for l in links(ln)]
    q['req_text'] = [clean(l[1:]) for l in req.splitlines() if l.startswith('*') and not l.startswith('**')]
    # objectives
    objs = []
    for l in sec.get('Objectives', '').splitlines():
        if l.startswith('*') and not l.startswith('**'):
            raw = l.lstrip('* ').strip()
            objs.append({'text': clean(raw), 'links': links(raw),
                         'fir': bool(re.search(r'in raid', raw, re.I))})
    q['objectives'] = objs
    # rewards
    rew = {'exp': None, 'roubles': None, 'rep': [], 'items': [], 'other': []}
    for l in sec.get('Rewards', '').splitlines():
        if not l.startswith('*') or l.startswith('**'): continue
        raw = l.lstrip('* ').strip(); c = clean(raw)
        if re.search(r'\bEXP\b', c): rew['exp'] = num(c)
        elif re.search(r'\bRep\b', c): rew['rep'].append(c)
        elif re.search(r'Roubles|Dollars|Euros', c) and links(raw) == []: rew['roubles'] = c
        elif re.match(r'\d+\s*[×x]', c): rew['items'].append(c)
        else: rew['other'].append(c)
    q['rewards'] = rew
    # guide table "Related Quest Items"
    items = []
    for row in table_rows(sec.get('Guide', '')):
        if len(row) >= 5 and links(row[1]):
            items.append({'item': links(row[1])[0], 'amount': num(row[2]), 'req': clean(row[3]), 'fir': clean(row[4])})
    q['quest_items'] = items
    return q

# ---------- ammo ----------
def parse_caliber_table(text):
    sec = sections(text).get('Types', '')
    keys = ['dmg','pen','armor_dmg','accuracy','recoil','light_bleed','heavy_bleed','dur_burn','heat','speed','source']
    out = []
    for row in table_rows(sec)[1:]:
        if len(row) < 13: continue
        d = {'name': links(row[1])[0] if links(row[1]) else clean(row[1]),
             'tracer': 'sup>T' in row[1], 'subsonic': 'sup>S' in row[1]}
        for k, v in zip(keys, row[2:]):
            d[k] = clean(v) if k == 'source' else num(v)
        out.append(d)
    return out

def parse_ammo_page(text):
    ib = infobox(text, 'Infobox ammo')
    keep = ['type','caliber','weight','grid','trader','velocity','damage','projectiles','penetration','armor damage',
            'accuracy','recoil','ricochet','l bleed chance','h bleed chance','durability burn','heat','effects','node','id']
    return {k: clean(ib[k]) for k in keep if k in ib and clean(ib[k])}

def parse_ammunition_index(text):
    out = {}
    for name, body in sections(text).items():
        for row in table_rows(body)[1:]:
            if len(row) == 3:
                cal = links(row[1])[0]
                out[cal] = [{'cls': (re.match(r'([A-Za-z\- ]+):', p.strip()) or [None, None])[1], 'weapon': w}
                            for p in row[2].split('<br/>') for w in links(p)]
    return out

# ---------- provisions / medical ----------
def parse_provisions(text):
    out = []
    for row in table_rows(text)[1:]:
        if len(row) < 9 or not links(row[1]): continue
        out.append({'name': links(row[1])[0], 'energy': num(row[2]), 'hydration': num(row[3]),
                    'buffs': clean(row[4]), 'debuffs': clean(row[5]), 'use_time': num(row[6]),
                    'barter': '✔' in row[7], 'craft': '✔' in row[8],
                    'quest_notes': [clean(l) for l in (row[9].splitlines() if len(row) > 9 else []) if l.startswith('*')]})
    return out

def parse_medical(text):
    out = []
    for name, body in sections(text).items():
        if name not in ('Medical items', 'Injectors and stimulants'): continue
        for row in table_rows(body)[1:]:
            if len(row) < 6 or not links(row[1]): continue
            out.append({'group': name, 'name': links(row[1])[0], 'type': clean(row[2]),
                        'buffs': clean(row[3]), 'debuffs': clean(row[4]),
                        'cols': [num(c) for c in row[5:]]})
    return out
