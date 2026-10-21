"""Rebuild repository-owned MIT reference packages with the portable SDK."""
from pathlib import Path
from copy import deepcopy
import json
import rchub_plugin as sdk

RELEASE_VERSION = '0.3.0'
SOURCE_ROOT = f'https://github.com/CetTek/rchub-plugins/tree/v{RELEASE_VERSION}/plugins/official'
PACKAGE_VERSIONS = {'system': '0.3.1'}


def label(en, tr):
    return {'en': en, 'tr': tr}


def string(n):
    return {'type': 'string', 'maxLength': n}


def integer(lo, hi):
    return {'type': 'integer', 'minimum': lo, 'maximum': hi}


def obj(fields, required=None):
    return {'type': 'object', 'properties': fields, 'required': list(fields) if required is None else required}


def action(name, entry, cap, inputs, outputs):
    return {'id': name, 'entry': entry, 'capability': cap, 'input': obj(inputs), 'output': obj(outputs)}


def manifest(name, tr, icon, color, permissions, actions, state, controls):
    m = sdk.template('com.rchub.' + name, name.title(), True)
    descriptions = {
        'system': ('Check CPU, memory and uptime.', 'CPU, bellek ve çalışma süresini görün.'),
        'browser': ('Open an HTTP(S) website on your PC.', 'PC’nizde HTTP(S) web sitesi açın.'),
        'power': ('Lock, sleep, restart or shut down your PC.', 'PC’nizi kilitleyin, uyutun, yeniden başlatın veya kapatın.'),
        'keyboard': ('Type text and send keyboard shortcuts.', 'Metin yazın ve klavye kısayolları gönderin.'),
        'media': ('Play, pause and change media tracks.', 'Medyayı oynatın, duraklatın ve parça değiştirin.'),
        'clipboard': ('Read and replace PC clipboard text.', 'PC panosundaki metni okuyun ve değiştirin.'),
        'apps': ('List running processes and stop a selected PID.', 'Çalışan işlemleri listeleyin ve seçtiğiniz PID’yi sonlandırın.'),
        'display': ('Adjust primary-monitor brightness with DDC/CI support.', 'DDC/CI destekli birincil monitörün parlaklığını ayarlayın.'),
        'screenshot': ('Capture an optimized primary-screen image.', 'Birincil ekranın iyileştirilmiş görüntüsünü alın.'),
        'speech': ('Speak short text on your PC, up to 10 seconds.', 'PC’nizde kısa metni en fazla 10 saniye seslendirin.'),
        'effects': ('Play a short Windows system sound.', 'Kısa bir Windows sistem sesi çalın.'),
    }
    m.update(name=label(name.title(), tr),
             description=label(*descriptions[name]),
             permissions=sorted(permissions), actions=actions, state=state,
             source=f'{SOURCE_ROOT}/{name}',
             ui={'version': 1, 'icon': icon, 'color': color, 'controls': controls})
    return m


def packages():
    result = {'audio': sdk.template('com.rchub.audio', 'Audio', True)}
    system = {'os': string(120), 'hostname': string(120), 'cpu_usage': integer(0, 100),
              'ram_used_mb': integer(0, 2147483647), 'ram_total_mb': integer(0, 2147483647),
              'ram_usage_percent': integer(0, 100),
              'uptime_seconds': integer(0, 2147483647)}
    result['system'] = manifest('system', 'Sistem Bilgisi', 'cpu', 'purple', ['system.read'],
        [action('read', 0, 'system.read', {}, system)], system,
        [{'id': 'read', 'type': 'button', 'label': label('Refresh', 'Yenile'), 'action': 'read'}] +
        [{'id': k, 'type': 'value', 'bind': k, 'label': label(en, tr)} for k, en, tr in [
            ('os', 'Operating system', 'İşletim sistemi'), ('hostname', 'Computer', 'Bilgisayar'),
            ('cpu_usage', 'CPU (%)', 'CPU (%)')]] + [
        {'id': 'ram_usage', 'type': 'progress', 'bind': 'ram_usage_percent',
         'min': 0, 'max': 100,
         'label': label('Memory usage', 'Bellek kullanımı')},
        {'id': 'ram_used_mb', 'type': 'value', 'bind': 'ram_used_mb',
         'label': label('Used memory (MB)', 'Kullanılan bellek (MB)')},
        {'id': 'ram_total_mb', 'type': 'value', 'bind': 'ram_total_mb',
         'label': label('Total memory (MB)', 'Toplam bellek (MB)')},
        {'id': 'uptime_seconds', 'type': 'value', 'bind': 'uptime_seconds',
         'label': label('Uptime (seconds)', 'Çalışma süresi (saniye)')}])
    result['browser'] = manifest('browser', 'Web Tarayıcısı', 'globe', 'orange', ['browser.open'],
        [action('open', 0, 'browser.open', {'url': string(2048)}, {'opened': {'type': 'boolean'}})],
        {'opened': {'type': 'boolean'}}, [
            {'id': 'url', 'type': 'input', 'label': label('Open website on PC', 'PC’de web sitesi aç'),
             'placeholder': label('https://example.com', 'https://example.com'), 'action': 'open', 'argument': 'url'},
            {'id': 'opened', 'type': 'status', 'label': label('Browser accepted request', 'Tarayıcı isteği kabul etti'), 'bind': 'opened'}])
    state = {'accepted': {'type': 'boolean'}}
    choices = ['lock', 'sleep', 'shutdown', 'restart']
    result['power'] = manifest('power', 'Güç', 'power', 'orange', ['power.write'],
        [action('change', 0, 'power.write', {'action': string(8) | {'enum': choices}}, state)], state,
        [{'id': k, 'type': 'button', 'label': label(en, tr), 'action': 'change', 'argument': 'action', 'value': k,
          'tone': 'danger' if k in ('shutdown', 'restart') else 'default'} for k, en, tr in [
            ('lock', 'Lock', 'Kilitle'), ('sleep', 'Sleep', 'Uyku'), ('shutdown', 'Shut down', 'Kapat'), ('restart', 'Restart', 'Yeniden başlat')]])
    sent = {'sent': {'type': 'boolean'}}
    result['keyboard'] = manifest('keyboard', 'Klavye', 'keyboard', 'purple', ['keyboard.write'],
        [action('type_text', 0, 'keyboard.write', {'text': string(4096)}, sent),
         action('shortcut', 1, 'keyboard.write', {'keys': string(80)}, sent)], sent, [
            {'id': 'text', 'type': 'input', 'label': label('Type on PC', 'PC’de yaz'), 'action': 'type_text', 'argument': 'text'},
            {'id': 'shortcut', 'type': 'input', 'label': label('Shortcut (e.g. ctrl+a)', 'Kısayol (ör. ctrl+a)'), 'action': 'shortcut', 'argument': 'keys'},
            {'id': 'sent', 'type': 'status', 'label': label('Sent', 'Gönderildi'), 'bind': 'sent'}])
    result['media'] = manifest('media', 'Medya', 'play', 'green', ['media.write'],
        [action('control', 0, 'media.write', {'action': string(10) | {'enum': ['play_pause', 'next', 'previous']}}, state)], state,
        [{'id': k, 'type': 'button', 'label': label(en, tr), 'action': 'control', 'argument': 'action', 'value': k}
         for k, en, tr in [('previous', 'Previous', 'Önceki'), ('play_pause', 'Play / pause', 'Oynat / duraklat'), ('next', 'Next', 'Sonraki')]])
    clip = {'text': string(8192)}
    result['clipboard'] = manifest('clipboard', 'Pano', 'clipboard', 'teal', ['clipboard.read', 'clipboard.write'],
        [action('read', 0, 'clipboard.read', {}, clip), action('write', 1, 'clipboard.write', clip, clip)], clip, [
            {'id': 'read', 'type': 'button', 'label': label('Read PC clipboard', 'PC panosunu oku'), 'action': 'read'},
            {'id': 'text', 'type': 'value', 'label': label('PC clipboard', 'PC panosu'), 'bind': 'text'},
            {'id': 'write', 'type': 'input', 'label': label('Write PC clipboard', 'PC panosuna yaz'), 'action': 'write', 'argument': 'text'}])
    app_state = {'apps': {'type': 'array', 'maxItems': 64, 'items': obj({'pid': integer(1, 2147483647), 'name': string(120)})},
                 'count': integer(0, 64), 'accepted': {'type': 'boolean'}}
    result['apps'] = manifest('apps', 'Uygulamalar', 'apps', 'green', ['apps.read', 'apps.write'],
        [action('read', 0, 'apps.read', {}, {k: app_state[k] for k in ('apps', 'count')}),
         action('stop', 1, 'apps.write', {'pid': integer(1, 2147483647)}, state)], app_state, [
            {'id': 'read', 'type': 'button', 'label': label('Refresh processes', 'İşlemleri yenile'), 'action': 'read'},
            {'id': 'count', 'type': 'value', 'label': label('Listed processes (max 64)', 'Listelenen işlemler (en fazla 64)'), 'bind': 'count'},
            {'id': 'processes', 'type': 'list', 'label': label('Stop a process', 'Bir işlemi sonlandır'), 'bind': 'apps',
             'itemLabel': 'name', 'itemValue': 'pid', 'action': 'stop', 'argument': 'pid', 'tone': 'danger'},
            {'id': 'accepted', 'type': 'status', 'label': label('Stop requested', 'Sonlandırma istendi'), 'bind': 'accepted'}])
    display = {'brightness': integer(0, 100), 'available': {'type': 'boolean'}}
    result['display'] = manifest('display', 'Ekran Parlaklığı', 'sun', 'orange', ['display.read', 'display.write'],
        [action('read', 0, 'display.read', {}, display), action('set', 1, 'display.write', {'brightness': integer(0, 100)}, display)], display, [
            {'id': 'read', 'type': 'button', 'label': label('Refresh brightness', 'Parlaklığı yenile'), 'action': 'read'},
            {'id': 'available', 'type': 'status', 'label': label('Primary monitor supports DDC/CI', 'Birincil monitör DDC/CI destekliyor'), 'bind': 'available'},
            {'id': 'brightness', 'type': 'slider', 'label': label('Brightness (supported DDC/CI monitor)', 'Parlaklık (destekli DDC/CI monitör)'), 'bind': 'brightness',
             'action': 'set', 'argument': 'brightness', 'min': 0, 'max': 100, 'step': 1}])
    image = {'image': string(800000), 'width': integer(1, 4096), 'height': integer(1, 4096)}
    result['screenshot'] = manifest('screenshot', 'Ekran Önizlemesi', 'camera', 'pink', ['screenshot.read'],
        [action('capture', 0, 'screenshot.read', {}, image)], image, [
            {'id': 'capture', 'type': 'button', 'label': label('Capture primary screen', 'Birincil ekranı yakala'), 'action': 'capture'},
            {'id': 'image', 'type': 'image', 'label': label('Optimized image · tap to view', 'Optimize edilmiş görüntü · görüntülemek için dokun'), 'bind': 'image'}])
    result['speech'] = manifest('speech', 'Metni Seslendir', 'message', 'green', ['speech.write'],
        [action('speak', 0, 'speech.write', {'text': string(2000)}, state)], state, [
            {'id': 'text', 'type': 'input', 'label': label('Speak on PC (up to 10 seconds)', 'PC’de seslendir (en fazla 10 saniye)'), 'action': 'speak', 'argument': 'text'},
            {'id': 'accepted', 'type': 'status', 'label': label('Speech completed', 'Seslendirme tamamlandı'), 'bind': 'accepted'}])
    result['effects'] = manifest('effects', 'Ses Efekti', 'smile', 'pink', ['effects.write'],
        [action('beep', 0, 'effects.write', {'effect': string(4) | {'enum': ['beep']}}, state)], state, [
            {'id': 'beep', 'type': 'button', 'label': label('Play system beep', 'Sistem sesi çal'), 'action': 'beep', 'argument': 'effect', 'value': 'beep'}])
    # A combined package owns its layout and requests the exact union at install.
    # Existing single-purpose packages stay reproducible for compatibility.
    combined = deepcopy(result['media'])
    combined.update(
        name=label('Media & Display', 'Medya ve Ekran'),
        description=label('Volume, brightness and playback in one panel.',
                          'Ses, parlaklık ve oynatma tek panelde.'),
        permissions=sorted(set(result['audio']['permissions'] + result['display']['permissions'] + result['media']['permissions'])),
        actions=[], state={},
        ui={'version': 1, 'icon': 'volume', 'color': 'blue', 'controls': []})
    groups = {}
    for part in ('audio', 'display', 'media'):
        original = deepcopy(result[part])
        combined['state'].update(original['state'])
        names = {}
        for a in original['actions']:
            names[a['id']] = f'{part}_{a["id"]}'
            a.update(id=names[a['id']], entry=len(combined['actions']))
            combined['actions'].append(a)
        def rewrite(nodes):
            output = []
            for c in nodes:
                if c.get('action') == 'read':
                    continue  # Host initializes read actions; compact refresh stays in panel chrome.
                if c.get('action') in names:
                    c['action'] = names[c['action']]
                if 'children' in c:
                    c['children'] = rewrite(c['children'])
                output.append(c)
            return output
        groups[part] = rewrite(original['ui']['controls'])
    audio = groups['audio'][0]['children']
    for c in audio:
        c['icon'] = 'volume'
    playback = groups['media']
    for c in playback:
        c['icon'] = 'play'
        if c.get('value') == 'play_pause':
            c['tone'] = 'primary'
    brightness = next(c for c in groups['display'] if c['type'] == 'slider')
    brightness.update(label=label('Brightness', 'Parlaklık'), icon='sun')
    combined['ui']['controls'] = [
        {'id': 'audio_row', 'type': 'row', 'children': audio},
        brightness,
        {'id': 'playback_section', 'type': 'section', 'icon': 'play',
         'label': label('Playback Controls', 'Oynatma Kontrolleri'), 'children': [
             {'id': 'playback_row', 'type': 'row', 'children': playback}]}]
    result['media'] = combined
    result['power']['ui']['controls'].sort(key=lambda c: ['shutdown', 'restart', 'lock', 'sleep'].index(c['id']))
    for c in result['power']['ui']['controls']:
        c['icon'] = 'power'
        if c['id'] == 'restart':
            c['tone'] = 'primary'
    def decorate(nodes, icon):
        for c in nodes:
            if c['type'] in ('button', 'section', 'slider'):
                c.setdefault('icon', icon)
            if 'children' in c:
                decorate(c['children'], icon)
    for m in result.values():
        decorate(m['ui']['controls'], m['ui']['icon'])
    def without_refresh(nodes, actions):
        return [dict(c, children=without_refresh(c['children'], actions)) if 'children' in c else c
                for c in nodes if not (c['type'] == 'button' and
                any(a['id'] == c.get('action') and a['capability'] in ('audio.read', 'display.read', 'system.read') for a in actions))]
    for m in result.values():
        m['ui']['controls'] = without_refresh(m['ui']['controls'], m['actions'])
    system_controls = result['system']['ui']['controls']
    for c in system_controls:
        if c['id'] == 'ram_usage':
            c.update(appearance='ring', label=label('RAM', 'RAM'))
    cpu = next(c for c in system_controls if c['id'] == 'cpu_usage')
    cpu.update(type='progress', appearance='ring', min=0, max=100, label=label('CPU', 'CPU'))
    result['system']['ui']['controls'] = [
        {'id': 'gauges', 'type': 'row', 'children': [cpu, next(c for c in system_controls if c['id'] == 'ram_usage')]},
        {'id': 'memory', 'type': 'row', 'children': [c for c in system_controls if c['id'] in ('ram_used_mb','ram_total_mb')]},
        {'id': 'computer', 'type': 'row', 'children': [c for c in system_controls if c['id'] in ('os','hostname')]},
        next(c for c in system_controls if c['id'] == 'uptime_seconds')]
    keyboard = result['keyboard']['ui']['controls']
    keyboard[0].update(multiline=True, placeholder=label('Write a message or paste multiple lines', 'Bir ileti yazın veya çok satırlı metin yapıştırın'))
    hotkeys = [('copy','Copy','Kopyala','ctrl+c'), ('paste','Paste','Yapıştır','ctrl+v'), ('select_all','Select all','Tümünü seç','ctrl+a'), ('undo','Undo','Geri al','ctrl+z'), ('enter','Enter','Enter','enter'), ('escape','Escape','Escape','esc'), ('switch','Switch app','Uygulama değiştir','alt+tab'), ('desktop','Show desktop','Masaüstünü göster','win+d')]
    result['keyboard']['ui']['controls'] = [
        {'id':'keyboard_tabs','type':'tabs','children':[
            {'id':'typing','type':'section','label':label('Text','Metin'),'children':[keyboard[0]]},
            {'id':'shortcuts','type':'section','label':label('Shortcuts','Kısayollar'),'children':[
                {'id':'quick_keys','type':'row','children':[{'id':k,'type':'button','label':label(en,tr),'action':'shortcut','argument':'keys','value':value} for k,en,tr,value in hotkeys]},keyboard[1]]}]},keyboard[2]]
    browser = result['browser']['ui']['controls']
    browser[0]['inputMode'] = 'url'
    result['browser']['ui']['controls'] = [browser[0], {'id':'quick_sites','type':'section','label':label('Quick sites','Hızlı siteler'),'children':[
        {'id':k,'type':'button','label':label(en,en),'action':'open','argument':'url','value':url,'icon':'globe'} for k,en,url in [('google','Google','https://www.google.com'),('youtube','YouTube','https://www.youtube.com'),('github','GitHub','https://github.com')]]},browser[1]]
    power = result['power']
    change = power['actions'][0]
    change['input'] = obj({'action': string(15) | {'enum':['lock','sleep','shutdown','restart','cancel_shutdown']}, 'delay_seconds':integer(0,86400),'force':{'type':'boolean'}}, ['action'])
    for c in power['ui']['controls']:
        if c['id'] in ('shutdown','restart'):
            c['inputBindings'] = {'delay_seconds':'delay_seconds','force':'force'}
            c['confirm'] = {'title':label('Confirm power action','Güç işlemini onayla'),'message':label('This may close applications and lose unsaved work. Continue?','Uygulamalar kapanabilir ve kaydedilmemiş çalışmalar kaybolabilir. Devam edilsin mi?')}
    power['ui']['controls'] = [
        {'id':'schedule','type':'section','label':label('Shutdown / restart options','Kapatma / yeniden başlatma seçenekleri'),'children':[
            {'id':'delay_seconds','type':'input','label':label('Delay (seconds)','Gecikme (saniye)'),'action':'change','argument':'delay_seconds','inputMode':'numeric','defaultValue':30,'submit':False},
            {'id':'force','type':'input','label':label('Force applications to close','Uygulamaları zorla kapat'),'action':'change','argument':'force','defaultValue':False,'submit':False}]},
        {'id':'power_actions','type':'column','children':power['ui']['controls']},
        {'id':'cancel_shutdown','type':'button','label':label('Cancel scheduled shutdown','Planlanan kapatmayı iptal et'),'action':'change','argument':'action','value':'cancel_shutdown','icon':'power'}]
    apps = result['apps']
    process = obj({'pid':integer(1,2147483647),'name':string(120),'title':string(240),'window_id':string(32),'started_at':integer(0,9007199254740991),'foreground':{'type':'boolean'}})
    apps['state']['apps']['items'] = process
    apps['actions'][0]['input'] = obj({'filter':string(10)|{'enum':['all','foreground']}},[])
    apps['actions'][0]['output']['properties']['apps']['items'] = process
    apps['actions'][1]['input'] = obj({'pid':integer(1,2147483647),'action':string(14)|{'enum':['stop','close','bring_to_front','minimize','maximize','restore']},'window_id':string(32),'started_at':integer(0,9007199254740991)},['pid','action','started_at'])
    item_actions=[]
    for value,en,tr in [('bring_to_front','Bring to front','Öne getir'),('minimize','Minimize','Küçült'),('maximize','Maximize','Büyüt'),('restore','Restore','Geri yükle'),('close','Close window','Pencereyi kapat'),('stop','Stop process','İşlemi sonlandır')]:
        row={'label':label(en,tr),'action':'stop','argument':'pid','args':{'action':value},'itemArguments':{'window_id':'window_id','started_at':'started_at'}}
        if value in ('close','stop'):
            row.update(tone='danger',confirm={'title':label(en,tr),'message':label('Unsaved work may be lost. Continue?','Kaydedilmemiş çalışmalar kaybolabilir. Devam edilsin mi?')})
        item_actions.append(row)
    apps['ui']['controls'] = [
        {'id':'filter','type':'select','label':label('Process view','İşlem görünümü'),'action':'read','argument':'filter','options':[{'label':label('Applications','Uygulamalar'),'value':'foreground'},{'label':label('All processes','Tüm işlemler'),'value':'all'}]},
        {'id':'processes','type':'list','label':label('Select an application','Bir uygulama seçin'),'bind':'apps','itemLabel':'name','itemValue':'pid','searchable':True,'itemActions':item_actions},
        {'id':'count','type':'value','label':label('Listed processes (max 64)','Listelenen işlemler (en fazla 64)'),'bind':'count'}]
    result['clipboard']['ui']['controls'][-1]['multiline'] = True
    return result


def build():
    for name, m in packages().items():
        version = PACKAGE_VERSIONS.get(name, RELEASE_VERSION)
        m.update(version=version, source=f'https://github.com/CetTek/rchub-plugins/tree/v{version}/plugins/official/{name}')
        root = sdk.ROOT / 'plugins/official' / name
        root.mkdir(parents=True, exist_ok=True)
        sdk.validate_manifest(m)
        (root / 'manifest.json').write_text(
            json.dumps(m, ensure_ascii=False, sort_keys=True, indent=2) + '\n',
            encoding='utf-8', newline='\n')
        (root / 'guest.wat').write_text(sdk.proxy_wat(m), encoding='utf-8', newline='\n')
        bundle = {'formatVersion': 1, 'manifest': m, 'wasm': list(sdk.proxy_wasm(m))}
        sdk.validate_bundle(bundle)
        (root / f'{name}.rchub-plugin.json').write_bytes(sdk.canonical(bundle))
        if not (root / 'README.md').exists():
            (root / 'README.md').write_text(
                f'# {m["name"]["en"]}\n\nMIT reference plugin. See manifest.json and guest.wat for full source.\n'
                'Rebuild from repository root with `python tools/plugins/build_official.py`.\n'
                'Installed grants are never inherited from another package or release.\n', encoding='utf-8', newline='\n')
        print(name, sdk.identity(bundle)['sha256'])


if __name__ == '__main__':
    build()
