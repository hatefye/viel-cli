(() => {
  const T = window.__TAURI__;
  const invoke = (cmd, args) => T.core.invoke(cmd, args);
  const listen = (name, fn) => T.event.listen(name, e => fn(e.payload));

  const api = {
    get: (path, query) => invoke('api', { method: 'GET', path, query: query || {} }),
    post: (path, body) => invoke('api', { method: 'POST', path, body: body || {} }),
  };

  const $ = (sel, root) => (root || document).querySelector(sel);
  const $$ = (sel, root) => Array.from((root || document).querySelectorAll(sel));
  const esc = s => String(s == null ? '' : s).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
  const el = html => { const t = document.createElement('template'); t.innerHTML = html.trim(); return t.content.firstElementChild; };
  const clamp = (v, a, b) => Math.min(Math.max(v, a), b);
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const log = (tag, msg) => invoke('log', { tag, msg: String(msg) }).catch(() => {});

  const clock = s => {
    s = Math.max(0, Math.floor(s || 0));
    const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), x = s % 60;
    return h ? `${h}:${String(m).padStart(2, '0')}:${String(x).padStart(2, '0')}` : `${m}:${String(x).padStart(2, '0')}`;
  };
  const human = s => { s = Math.round(s || 0); const h = Math.floor(s / 3600), m = Math.round((s % 3600) / 60); return h ? `${h}h ${m}m` : `${m}m`; };
  const size = b => b >= 1073741824 ? `${(b / 1073741824).toFixed(1)} GB` : `${Math.round(b / 1048576)} MB`;

  const P = {
    home: 'M3 11l9-8 9 8M5 10v10h14V10M10 20v-6h4v6',
    search: 'M11 4a7 7 0 1 1 0 14 7 7 0 0 1 0-14zM20 20l-4-4',
    bookmark: 'M6 3h12v18l-6-4-6 4z',
    clock: 'M12 3a9 9 0 1 1 0 18 9 9 0 0 1 0-18zM12 7v5l3 2',
    download: 'M12 3v12M7 10l5 5 5-5M4 19h16',
    sliders: 'M4 6h10M18 6h2M4 12h3M11 12h9M4 18h12M20 18h0M14 4v4M7 10v4M16 16v4',
    user: 'M12 3a4 4 0 1 1 0 8 4 4 0 0 1 0-8zM4 21a8 8 0 0 1 16 0',
    back: 'M15 5l-7 7 7 7',
    list: 'M4 6h16M4 12h16M4 18h16',
    play: 'M7 4l12 8-12 8z',
    pause: 'M7 4h3v16H7zM14 4h3v16h-3z',
    rew: 'M12 4a8 8 0 1 1-7.4 11M4 4v6h6',
    ffw: 'M12 4a8 8 0 1 0 7.4 11M20 4v6h-6',
    vol: 'M4 9v6h4l5 4V5L8 9zM16 9a4 4 0 0 1 0 6M18.5 6.5a8 8 0 0 1 0 11',
    volx: 'M4 9v6h4l5 4V5L8 9zM16 9l5 6M21 9l-5 6',
    cc: 'M3 6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM10 10H8a2 2 0 0 0 0 4h2M16 10h-2a2 2 0 0 0 0 4h2',
    audio: 'M4 12a8 8 0 0 1 16 0v5a2 2 0 0 1-2 2h-1v-6h3M4 12v5a2 2 0 0 0 2 2h1v-6H4',
    quality: 'M3 5h18v12H3zM8 21h8M8 9v4M8 11h3M11 9v4M14 9h2a2 2 0 0 1 0 4h-2z',
    next: 'M5 4l10 8-10 8zM18 4v16',
    pip: 'M3 5h18v14H3zM12 11h7v5h-7z',
    full: 'M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5',
    unfull: 'M9 4v5H4M20 9h-5V4M15 20v-5h5M4 15h5v5',
    check: 'M5 12l5 5 9-10',
    x: 'M6 6l12 12M18 6L6 18',
    plus: 'M12 5v14M5 12h14',
    info: 'M12 3a9 9 0 1 1 0 18 9 9 0 0 1 0-18zM12 11v5M12 8h0',
    left: 'M15 5l-7 7 7 7',
    right: 'M9 5l7 7-7 7',
    share: 'M4 12v7a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-7M12 15V3M7 8l5-5 5 5',
    trash: 'M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v6M14 11v6',
    folder: 'M3 6h6l2 2h10v11H3z',
    stats: 'M4 20V10M10 20V4M16 20v-8M22 20H2',
    moon: 'M20 15a8 8 0 0 1-11-11 8 8 0 1 0 11 11z',
    star: 'M12 3l2.7 5.8 6.3.8-4.6 4.4 1.2 6.3L12 17.2 6.4 20.3l1.2-6.3L3 9.6l6.3-.8z',
    phone: 'M7 2h10a2 2 0 0 1 2 2v16a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2zM11 19h2',
    refresh: 'M20 12a8 8 0 1 1-2.3-5.7M20 4v5h-5',
    offline: 'M4 16a4 4 0 0 1 1-7.9A6 6 0 0 1 16.6 7 5 5 0 0 1 19 16.5M12 12v9M8.5 17.5L12 21l3.5-3.5',
  };
  const icon = (name, filled) => `<svg viewBox="0 0 24 24" fill="${filled ? 'currentColor' : 'none'}" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="${P[name] || ''}"/></svg>`;
  const FILLED = new Set(['play', 'pause', 'next', 'bookmark-on', 'star']);
  const ic = (name, cls) => `<i class="ic ${cls || ''}">${icon(name, FILLED.has(name))}</i>`;
  const hydrateIcons = root => $$('.ic[data-icon]', root).forEach(i => { i.innerHTML = icon(i.dataset.icon, FILLED.has(i.dataset.icon)); i.removeAttribute('data-icon'); });

  let toastTimer;
  const toast = text => {
    const t = $('#toast');
    t.textContent = text;
    t.classList.add('show');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => t.classList.remove('show'), 2600);
  };

  const Settings = {
    data: Object.assign({
      maxHeight: 1080, downloadHeight: 1080, subtitleLang: 'en', audioLang: '', subSize: 1, skipSeconds: 10,
      autoplay: true, hwdec: true, stats: false, volume: 100, mute: false, speed: 1, subBackground: true,
    }, JSON.parse(localStorage.getItem('settings') || '{}')),
    save() { localStorage.setItem('settings', JSON.stringify(this.data)); },
    set(k, v) { this.data[k] = v; this.save(); },
    push() {
      return api.post('settings', { max_height: this.data.maxHeight, subtitle_lang: this.data.subtitleLang, audio_lang: this.data.audioLang || '' }).catch(() => {});
    },
  };
  const S = Settings.data;

  const Memory = {
    map(k) { try { return JSON.parse(localStorage.getItem(k) || '{}'); } catch (_) { return {}; } },
    store(k, v) { localStorage.setItem(k, JSON.stringify(v)); },
    titleKey: t => `${t.kind}-${t.id}`,
    preferredSource(t) { return this.map('sources')[this.titleKey(t)]; },
    rememberSource(t, src) { const m = this.map('sources'); m[this.titleKey(t)] = src; this.store('sources', m); },
    lastPlayed(t) { return this.map('last')[this.titleKey(t)]; },
    remember(p) {
      const m = this.map('last');
      m[`${p.kind}-${p.id}`] = { title: p.title, season: p.season, episode: p.episode, position: p.position, duration: p.duration, at: Date.now() };
      this.store('last', m);
      if (p.duration > 0 && p.duration - p.position < 30) this.markWatched(`${p.season}-${p.episode}`, `${p.kind}-${p.id}`);
    },
    watched(key) { return new Set(this.map('watched')[key] || []); },
    markWatched(ep, key) { const m = this.map('watched'); const s = new Set(m[key] || []); s.add(ep); m[key] = Array.from(s); this.store('watched', m); },
    subtitle(t) { return this.map('subs')[this.titleKey(t)]; },
    rememberSubtitle(t, lang) { const m = this.map('subs'); m[this.titleKey(t)] = lang == null ? 'off' : lang; this.store('subs', m); },
  };

  const LANGS = { en: 'English', es: 'Spanish', fr: 'French', de: 'German', pt: 'Portuguese', it: 'Italian', ja: 'Japanese', ar: 'Arabic', ru: 'Russian', ko: 'Korean', zh: 'Chinese', tr: 'Turkish', pl: 'Polish', nl: 'Dutch', sv: 'Swedish', el: 'Greek', ro: 'Romanian', hi: 'Hindi', th: 'Thai', vi: 'Vietnamese', id: 'Indonesian' };
  const langName = code => { const c = String(code || '').toLowerCase(); return LANGS[c] || LANGS[c.slice(0, 2)] || (code || 'Unknown'); };
  const SOURCE_NAMES = { vixsrc: 'Vix', cno: 'Cino', mapple: 'Mapple', vidbolt: 'Bolt', animekai: 'Kai', animeparadise: 'Paradise', luffy: 'Luffy', kototv: 'Koto', offline: 'Downloaded' };
  const sourceLabel = s => SOURCE_NAMES[s] || (s ? s[0].toUpperCase() + s.slice(1) : '');

  const kindLabel = k => k === 'movie' ? 'Movie' : k === 'anime' ? 'Anime' : 'Series';
  const numText = n => Number.isInteger(n) ? String(n) : String(n);
  const epTag = (ep, anime) => anime ? `E${numText(ep.number)}` : `S${ep.season} · E${Math.trunc(ep.number)}`;
  const epLabel = (ep, anime) => ep.name ? `${epTag(ep, anime)} · ${ep.name}` : epTag(ep, anime);

  const Item = {
    make(title, episodes, index, dub, startAt, chain, offline) {
      return { title, episodes: episodes || [], index: index || 0, dub: !!dub, startAt: startAt || null, chain: chain || [], offline: offline || null };
    },
    ep: it => it.episodes[it.index] || null,
    isAnime: it => it.title.kind === 'anime',
    isMovie: it => it.title.kind === 'movie',
    heading(it) { const e = this.ep(it); return e ? `${it.title.name} · ${epLabel(e, this.isAnime(it))}${it.dub ? ' (dub)' : ''}` : it.title.name; },
    tag(it) { const e = this.ep(it); return e ? epTag(e, this.isAnime(it)) + (it.dub ? ' · Dub' : '') : (it.title.year || kindLabel(it.title.kind)); },
    next(it) { if (this.isMovie(it) || !it.episodes[it.index + 1]) return null; return this.make(it.title, it.episodes, it.index + 1, it.dub, null, it.chain); },
    at(it, i) { return this.make(it.title, it.episodes, i, it.dub, null, it.chain); },
    request(it) { const e = this.ep(it); return { kind: it.title.kind, id: it.title.id, title: this.heading(it), season: e ? e.season : 0, episode: e ? e.number : 0, dub: it.dub, sources: e ? e.sources || [] : [] }; },
    progress(it, position, duration) {
      const e = this.ep(it);
      const name = this.isAnime(it) ? (e && e.name ? e.name : it.title.name) : it.title.name;
      return { kind: it.title.kind, id: it.title.id, season: e ? e.season : 0, episode: e ? Math.trunc(e.number) : 0, position, duration, title: name };
    },
    downloadKey(it) { const e = this.ep(it); if (!e) return `${it.title.kind}-${it.title.id}`; return `${it.title.kind}-${it.title.id}-${e.season}-${numText(e.number).replace('.', '_')}${it.dub ? '-dub' : ''}`; },
    downloadMeta(it) {
      const e = this.ep(it);
      return { name: it.title.name, label: e ? epLabel(e, this.isAnime(it)) + (it.dub ? ' (dub)' : '') : (it.title.year || kindLabel(it.title.kind)), kind: it.title.kind, id: it.title.id, season: e ? e.season : 0, episode: e ? e.number : 0, poster: it.title.poster, still: e ? e.still : null, dub: it.dub };
    },
  };

  const App = { boot: null, user: null, view: 'home', home: null, homeAt: 0, history: null, historyAt: 0, downloads: [], active: {}, watchlist: null };

  const nav = $('#side');
  const view = $('#view');

  function setView(name, opts) {
    App.view = name;
    $$('.nav', nav).forEach(b => b.classList.toggle('on', b.dataset.view === name));
    view.scrollTop = 0;
    const render = Views[name];
    if (render) render(opts || {});
  }

  nav.addEventListener('click', e => { const b = e.target.closest('.nav'); if (b) setView(b.dataset.view); });
  $$('.tb-btn').forEach(b => b.addEventListener('click', () => invoke('win', { action: b.dataset.win })));
  $('#titlebar').addEventListener('dblclick', e => { if (e.target.closest('.tb-btn')) return; invoke('win', { action: 'maximize' }); });

  const img = (src, cls) => src ? `<img ${cls ? `class="${cls}"` : ''} loading="lazy" src="${esc(src)}" onload="this.classList.add('ok')" onerror="this.remove()">` : '';
  const meta = t => {
    const bits = [];
    if (t.year) bits.push(`<span>${esc(t.year)}</span>`);
    if (t.rating > 0) bits.push(`<span class="star">${ic('star')} ${Number(t.rating).toFixed(1)}</span>`);
    bits.push(`<span>${kindLabel(t.kind)}</span>`);
    if (t.season_count > 1) bits.push(`<span>${t.season_count} seasons</span>`);
    else if (t.runtime) bits.push(`<span>${human(t.runtime * 60)}</span>`);
    (t.genres || []).slice(0, 3).forEach(g => bits.push(`<span class="muted">${esc(g)}</span>`));
    return `<div class="meta">${bits.join('<i class="dot"></i>')}</div>`;
  };

  function card(t, opts) {
    opts = opts || {};
    const c = el(`<div class="card ${opts.rank ? 'ranked' : ''} ${opts.wide ? 'wide' : ''}">
      <div class="img">${img(opts.wide ? (t.still || t.poster) : t.poster)}
        ${opts.progress != null ? `<div class="prog"><b style="width:${Math.round(opts.progress * 100)}%"></b></div>` : ''}
        ${opts.play ? `<div class="playbtn"><i>${ic('play')}</i></div>` : ''}
        ${opts.remove ? `<button class="x" title="Remove">${ic('x')}</button>` : ''}
      </div>
      ${opts.rank ? `<div class="rank">${opts.rank}</div>` : ''}
      <div class="t">${esc(t.name)}</div>
      <div class="s">${esc(opts.sub != null ? opts.sub : [t.year, t.rating > 0 ? `★ ${Number(t.rating).toFixed(1)}` : null].filter(Boolean).join(' · '))}</div>
    </div>`);
    c.addEventListener('click', e => {
      if (e.target.closest('.x')) { opts.remove(); return; }
      if (opts.play && e.target.closest('.playbtn')) { opts.play(); return; }
      if (opts.open) opts.open(); else openDetails(t.kind, t.id);
    });
    return c;
  }

  function row(title, items, build, sub) {
    if (!items || !items.length) return null;
    const r = el(`<section class="row"><h2>${esc(title)}${sub ? `<small>${esc(sub)}</small>` : ''}</h2><button class="arrow l">${ic('left')}</button><div class="strip"></div><button class="arrow r">${ic('right')}</button></section>`);
    const strip = $('.strip', r);
    items.forEach((it, i) => strip.appendChild(build(it, i)));
    $('.arrow.l', r).onclick = () => strip.scrollBy({ left: -strip.clientWidth * 0.8, behavior: 'smooth' });
    $('.arrow.r', r).onclick = () => strip.scrollBy({ left: strip.clientWidth * 0.8, behavior: 'smooth' });
    return r;
  }

  const Views = {};

  Views.home = async function () {
    if (!App.home || Date.now() - App.homeAt > 300000) {
      view.innerHTML = `<div class="page"><div class="hero skeleton" style="margin:-8px -32px 26px;border-radius:0"></div>${'<div class="row"><div class="skeleton" style="height:22px;width:180px;margin-bottom:14px"></div><div class="strip">' + '<div class="card"><div class="img skeleton"></div></div>'.repeat(8) + '</div></div>'.repeat(3)}</div>`;
      try { App.home = await api.get('home'); App.homeAt = Date.now(); } catch (e) { view.innerHTML = `<div class="page"><div class="empty"><b>Can't reach viel</b>${esc(e)}<br><br><button class="btn sm" id="retry">Try again</button></div></div>`; $('#retry').onclick = () => Views.home(); return; }
      if (App.view !== 'home') return;
    }
    const d = App.home;
    const page = el('<div class="page"></div>');
    page.appendChild(hero(d));
    const cont = (d.continue || []).filter(r => r.duration > 0);
    const add = r => { if (r) page.appendChild(r); };
    add(row('Continue watching', cont, r => {
      const t = { id: r.id, kind: r.kind, name: r.show, poster: r.poster };
      const left = Math.max(r.duration - r.position, 0);
      const label = r.kind === 'movie' ? `${clock(left)} left` : `${r.kind === 'anime' ? `E${r.episode}` : `S${r.season} · E${r.episode}`} · ${clock(left)} left`;
      return card(t, { progress: r.duration ? r.position / r.duration : 0, sub: label, play: () => resumeFromRow(t, r), remove: async () => { await api.post('history/remove', { kind: r.kind, id: r.id }).catch(() => {}); App.homeAt = 0; Views.home(); } });
    }));
    add(row('Top 10 today', d.top, (t, i) => card(t, { rank: i + 1 })));
    add(row('Trending this week', d.trending, t => card(t)));
    add(row('Anime', d.anime, t => card(t)));
    add(row('Popular series', d.popular_tv, t => card(t)));
    add(row('Top rated movies', d.top_rated, t => card(t)));
    view.innerHTML = '';
    view.appendChild(page);
  };

  const heroCache = {};
  let heroTimer;
  function hero(d) {
    const slides = [];
    if (d.hero) { slides.push(d.hero); heroCache[`${d.hero.kind}-${d.hero.id}`] = d.hero; }
    (d.top || []).forEach(t => { if (!slides.find(s => s.id === t.id && s.kind === t.kind) && slides.length < 6) slides.push(t); });
    const h = el('<div class="hero"><div class="bd"></div><div class="copy"></div><div class="dots"></div></div>');
    const dots = $('.dots', h);
    slides.forEach((_, i) => { const b = el('<b></b>'); b.onclick = () => show(i, true); dots.appendChild(b); });
    let idx = 0;
    async function show(i, manual) {
      idx = i;
      const t = slides[i];
      if (!t) return;
      $$('.dots b', h).forEach((b, j) => b.classList.toggle('on', j === i));
      const bd = $('.bd', h);
      bd.style.opacity = 0;
      setTimeout(() => { bd.style.backgroundImage = `url("${t.backdrop || t.poster || ''}")`; bd.style.transition = 'opacity .6s'; bd.style.opacity = 1; }, 180);
      let det = heroCache[`${t.kind}-${t.id}`];
      if (!det) { try { det = await api.get('details', { kind: t.kind, id: String(t.id) }); heroCache[`${t.kind}-${t.id}`] = det; } catch (_) { det = t; } if (idx !== i) return; }
      const copy = $('.copy', h);
      copy.innerHTML = `${det.logo ? `<img class="logo" src="${esc(det.logo)}" alt="">` : `<div class="name">${esc(det.name)}</div>`}
        ${meta(det)}
        <div class="over">${esc(det.overview || det.tagline || '')}</div>
        <div class="actions"><button class="btn primary" id="h-play">${ic('play')} Play</button><button class="btn" id="h-info">${ic('info')} More info</button></div>`;
      $('#h-play', copy).onclick = () => openDetails(t.kind, t.id, { autoplay: true });
      $('#h-info', copy).onclick = () => openDetails(t.kind, t.id);
      clearTimeout(heroTimer);
      heroTimer = setTimeout(() => { if (document.body.contains(h)) show((idx + 1) % slides.length); }, 9000);
    }
    show(0);
    return h;
  }

  async function resumeFromRow(t, r) {
    openDetails(t.kind, t.id, { autoplay: true, row: r });
  }

  Views.search = function (opts) {
    const page = el(`<div class="page"><div class="searchbar"><div class="field">${ic('search')}<input id="q" placeholder="Search movies, series, anime" autofocus></div></div><div id="results"></div></div>`);
    view.innerHTML = '';
    view.appendChild(page);
    const input = $('#q', page), results = $('#results', page);
    let timer, seq = 0;
    const idle = () => {
      results.innerHTML = '';
      const d = App.home;
      if (!d) return;
      const add = r => { if (r) results.appendChild(r); };
      add(row('Trending now', d.trending, t => card(t)));
      add(row('Anime', d.anime, t => card(t)));
    };
    const run = async () => {
      const q = input.value.trim();
      App.lastSearch = q;
      if (!q) return idle();
      const my = ++seq;
      let rows;
      try { rows = await api.get('search', { q }); } catch (e) { rows = []; }
      if (my !== seq) return;
      results.innerHTML = '';
      if (!rows.length) { results.innerHTML = `<div class="empty"><b>Nothing found</b>Try another spelling.</div>`; return; }
      const grid = el('<div class="grid"></div>');
      rows.forEach(t => grid.appendChild(card(t)));
      results.appendChild(grid);
    };
    input.addEventListener('input', () => { clearTimeout(timer); timer = setTimeout(run, 280); });
    input.value = App.lastSearch || '';
    run();
    setTimeout(() => input.focus(), 30);
  };

  Views.list = async function () {
    view.innerHTML = '<div class="page"><h1>My List</h1><div class="grid">' + '<div class="card"><div class="img skeleton"></div></div>'.repeat(6) + '</div></div>';
    let rows = [];
    try { rows = await api.get('watchlist'); } catch (e) { toast(String(e)); }
    App.watchlist = rows;
    if (App.view !== 'list') return;
    const page = el('<div class="page"><h1>My List</h1></div>');
    if (!rows.length) page.appendChild(el('<div class="empty"><b>Nothing saved yet</b>Use the bookmark on any title to keep it here.</div>'));
    else { const grid = el('<div class="grid"></div>'); rows.forEach(t => grid.appendChild(card(t, { sub: kindLabel(t.kind), remove: async () => { await api.post('watchlist/remove', { kind: t.kind, id: t.id }).catch(() => {}); Views.list(); } }))); page.appendChild(grid); }
    view.innerHTML = '';
    view.appendChild(page);
  };

  async function loadHistory(force) {
    if (!force && App.history && Date.now() - App.historyAt < 45000) return App.history;
    try { App.history = await api.get('history'); App.historyAt = Date.now(); } catch (_) { App.history = App.history || []; }
    return App.history;
  }

  Views.history = async function () {
    view.innerHTML = '<div class="page"><h1>History</h1><div class="list">' + '<div class="li"><div class="poster skeleton"></div><div><div class="skeleton" style="height:14px;width:220px"></div></div></div>'.repeat(6) + '</div></div>';
    const rows = await loadHistory(true);
    if (App.view !== 'history') return;
    const page = el('<div class="page"><h1>History</h1></div>');
    if (!rows.length) page.appendChild(el('<div class="empty"><b>No history yet</b>Everything you watch shows up here, synced with the site.</div>'));
    const list = el('<div class="list"></div>');
    rows.forEach(r => {
      const t = { id: r.id, kind: r.kind, name: r.show, poster: r.poster };
      const tag = r.kind === 'movie' ? '' : r.kind === 'anime' ? `E${r.episode}` : `S${r.season} · E${r.episode}`;
      const done = r.duration > 0 && r.duration - r.position < 30;
      const li = el(`<div class="li"><div class="poster">${img(r.poster)}</div><div><div class="n">${esc(r.show)}</div><div class="s">${[tag, done ? 'Finished' : r.duration ? `${clock(Math.max(r.duration - r.position, 0))} left` : '', new Date(r.updated_at * 1000).toLocaleDateString()].filter(Boolean).join(' · ')}</div>${r.duration ? `<div class="bar"><b style="width:${Math.round(r.position / r.duration * 100)}%"></b></div>` : ''}</div>
        <div class="acts"><button class="rb" title="Play">${ic('play')}</button><button class="rb" title="Remove">${ic('trash')}</button></div></div>`);
      li.addEventListener('click', async e => {
        if (e.target.closest('[title=Remove]')) { await api.post('history/remove', { kind: r.kind, id: r.id }).catch(() => {}); li.remove(); App.homeAt = 0; return; }
        openDetails(r.kind, r.id, { autoplay: !!e.target.closest('[title=Play]'), row: r });
      });
      list.appendChild(li);
    });
    page.appendChild(list);
    view.innerHTML = '';
    view.appendChild(page);
  };

  Views.downloads = async function () {
    let rows = [];
    try { rows = await api.get('downloads'); } catch (_) {}
    App.downloads = rows;
    if (App.view !== 'downloads') return;
    const page = el(`<div class="page"><h1>Downloads <button class="btn ghost sm" id="dl-folder" style="margin-left:8px;vertical-align:middle">${ic('folder')} Open folder</button></h1></div>`);
    $('#dl-folder', page).onclick = () => invoke('open_downloads');
    if (!rows.length) page.appendChild(el('<div class="empty"><b>No downloads</b>Use the download button on any episode or movie. They play here without a connection.</div>'));
    const list = el('<div class="list"></div>');
    rows.forEach(r => list.appendChild(downloadRow(r)));
    page.appendChild(list);
    view.innerHTML = '';
    view.appendChild(page);
  };

  function downloadRow(r) {
    const m = r.meta || {};
    const live = App.active[r.key];
    const state = live ? live.state : r.state;
    const done = live ? live.done : r.done, total = live ? live.total : r.total;
    const frac = total ? done / total : 0;
    const status = state === 'done' ? `${size(r.bytes)} · ${sourceLabel(r.source)}` : state === 'failed' ? `Failed · ${r.error || ''}` : state === 'resolving' ? 'Finding a source…' : state && state.startsWith('failed') ? state : total ? `${Math.round(frac * 100)}% · ${done}/${total} parts` : 'Starting…';
    const li = el(`<div class="li" data-key="${esc(r.key)}"><div class="poster">${img(m.poster)}</div><div><div class="n">${esc(m.name || r.key)}</div><div class="s">${esc(m.label || '')} · ${esc(status)}</div>${state !== 'done' && !String(state).startsWith('failed') && state !== 'failed' ? `<div class="bar"><b style="width:${Math.round(frac * 100)}%"></b></div>` : ''}</div>
      <div class="acts">${state === 'done' ? `<button class="rb" title="Play">${ic('play')}</button>` : ''}<button class="rb" title="Remove">${ic(state === 'done' || state === 'failed' ? 'trash' : 'x')}</button></div></div>`);
    li.addEventListener('click', async e => {
      if (e.target.closest('[title=Remove]')) {
        if (App.active[r.key]) invoke('download_cancel', { key: r.key });
        else await api.post('downloads/remove', { key: r.key }).catch(() => {});
        li.remove();
        return;
      }
      if (state === 'done') playOffline(r);
    });
    return li;
  }

  function playOffline(r) {
    const m = r.meta || {};
    const title = { id: m.id, kind: m.kind, name: m.name, poster: m.poster, year: '' };
    const episodes = m.kind === 'movie' ? [] : [{ season: m.season || 0, number: m.episode || 0, name: m.label || '', overview: '', still: m.still, sources: [] }];
    const item = Item.make(title, episodes, 0, !!m.dub, null, [], r.key);
    Player.start(item);
  }

  Views.settings = function () {
    const page = el('<div class="page"><h1>Settings</h1><div class="settings"></div></div>');
    const box = $('.settings', page);
    const seg = (key, options, fmt, after) => {
      const s = el(`<div class="seg">${options.map(o => `<button data-v="${o}" class="${S[key] == o ? 'on' : ''}">${fmt ? fmt(o) : o}</button>`).join('')}</div>`);
      s.addEventListener('click', e => { const b = e.target.closest('button'); if (!b) return; const v = isNaN(Number(b.dataset.v)) ? b.dataset.v : Number(b.dataset.v); Settings.set(key, v); $$('button', s).forEach(x => x.classList.toggle('on', x === b)); if (after) after(v); });
      return s;
    };
    const sw = (key, after) => { const s = el(`<button class="sw ${S[key] ? 'on' : ''}" role="switch"></button>`); s.onclick = () => { Settings.set(key, !S[key]); s.classList.toggle('on', S[key]); if (after) after(S[key]); }; return s; };
    const select = (key, options, after) => { const s = el(`<select>${options.map(([v, l]) => `<option value="${v}" ${S[key] == v ? 'selected' : ''}>${l}</option>`).join('')}</select>`); s.onchange = () => { Settings.set(key, s.value); if (after) after(s.value); }; return s; };
    const rowEl = (label, hint, control) => { const r = el(`<div class="srow"><div class="lbl"><b>${label}</b>${hint ? `<small>${hint}</small>` : ''}</div></div>`); r.appendChild(control); return r; };
    const group = (title, rows) => { const g = el(`<div class="group glass-card"><h3>${title}</h3></div>`); rows.forEach(r => g.appendChild(r)); return g; };
    const q = v => v === 2160 ? '4K' : `${v}p`;
    const langs = [['en', 'English'], ['es', 'Spanish'], ['fr', 'French'], ['de', 'German'], ['pt', 'Portuguese'], ['it', 'Italian'], ['ja', 'Japanese'], ['ar', 'Arabic'], ['ru', 'Russian'], ['ko', 'Korean'], ['zh', 'Chinese'], ['tr', 'Turkish'], ['pl', 'Polish'], ['nl', 'Dutch'], ['sv', 'Swedish'], ['hi', 'Hindi']];
    box.appendChild(group('Playback', [
      rowEl('Streaming quality', 'Highest rendition the relay hands to the player', seg('maxHeight', [480, 720, 1080, 2160], q, () => Settings.push())),
      rowEl('Hardware decoding', 'Uses the GPU for video. Turn off if you see glitches', sw('hwdec', v => Player.set('hwdec', v ? 'auto-safe' : 'no'))),
      rowEl('Autoplay next episode', 'Starts the next one with a short countdown', sw('autoplay')),
      rowEl('Skip buttons', 'Seconds for the arrow keys and skip buttons', seg('skipSeconds', [10, 15, 30], v => `${v}s`, () => Player.refreshSkip())),
    ]));
    box.appendChild(group('Subtitles and audio', [
      rowEl('Subtitle language', 'Picked automatically when a source has it', select('subtitleLang', langs, () => Settings.push())),
      rowEl('Audio language', 'For sources with several audio tracks', select('audioLang', [['', 'Match subtitles']].concat(langs), () => Settings.push())),
      rowEl('Subtitle size', '', seg('subSize', [0.8, 1, 1.2, 1.5], v => v === 1 ? 'Normal' : v < 1 ? 'Small' : v < 1.4 ? 'Large' : 'Huge', () => Player.applySubStyle())),
      rowEl('Subtitle backdrop', 'Soft shadow box behind the text', sw('subBackground', () => Player.applySubStyle())),
    ]));
    box.appendChild(group('Downloads', [
      rowEl('Download quality', '', seg('downloadHeight', [480, 720, 1080, 2160], q)),
      rowEl('Folder', App.boot && App.boot.downloads ? esc(App.boot.downloads) : '', (() => { const b = el(`<button class="btn sm">${ic('folder')} Open</button>`); b.onclick = () => invoke('open_downloads'); return b; })()),
    ]));
    box.appendChild(el(`<div class="group glass-card"><h3>Keyboard</h3><div class="keys">
      <div><span>Play / pause</span><span><kbd>Space</kbd> <kbd>K</kbd></span></div><div><span>Seek</span><span><kbd>←</kbd> <kbd>→</kbd></span></div>
      <div><span>Seek 1 minute</span><span><kbd>Shift</kbd>+<kbd>←</kbd>/<kbd>→</kbd></span></div><div><span>Volume</span><span><kbd>↑</kbd> <kbd>↓</kbd></span></div>
      <div><span>Mute</span><span><kbd>M</kbd></span></div><div><span>Fullscreen</span><span><kbd>F</kbd></span></div>
      <div><span>Subtitles on/off</span><span><kbd>C</kbd></span></div><div><span>Next episode</span><span><kbd>N</kbd></span></div>
      <div><span>Speed</span><span><kbd>[</kbd> <kbd>]</kbd></span></div><div><span>Stats</span><span><kbd>I</kbd></span></div>
      <div><span>Episodes</span><span><kbd>E</kbd></span></div><div><span>Mini player</span><span><kbd>P</kbd></span></div>
      <div><span>Back / close</span><span><kbd>Esc</kbd></span></div><div><span>Search</span><span><kbd>Ctrl</kbd>+<kbd>K</kbd></span></div>
    </div></div>`));
    view.innerHTML = '';
    view.appendChild(page);
  };

  Views.account = function () {
    const b = App.boot || {};
    const page = el(`<div class="page account"><h1>Account</h1>
      <div class="group glass-card" style="padding:18px 20px"><div class="big">${esc(App.user || 'Not signed in')}</div><div class="muted">${esc(b.host || 'https://viel.tv')}</div>
        <div class="actions" style="margin-top:16px"><button class="btn" id="acc-site">Open viel.tv</button><button class="btn ghost" id="acc-out">Sign out</button></div></div>
      <div class="group glass-card" style="margin-top:10px">
        <div class="srow"><div class="lbl"><b>Server</b><small>Point the app at a mirror if viel.tv is blocked for you</small></div><input type="text" id="acc-host" value="${esc(b.host || '')}" spellcheck="false"></div>
        <div class="srow"><div class="lbl"><b>Version</b><small>viel for Windows</small></div><span class="muted mono">${esc(b.version || '')}</span></div>
        <div class="srow"><div class="lbl"><b>Player</b><small>${b.mpv ? esc(b.mpv) : 'libmpv loaded'}</small></div><span class="muted">${b.mpv ? '⚠︎' : '✓'}</span></div>
      </div></div>`);
    $('#acc-site', page).onclick = () => invoke('open_external', { url: b.host || 'https://viel.tv' });
    $('#acc-out', page).onclick = async () => { await api.post('logout').catch(() => {}); App.user = null; App.home = null; showLogin(); };
    $('#acc-host', page).onchange = async e => { await invoke('set_host', { host: e.target.value }); App.boot.host = e.target.value; App.home = null; toast('Server updated'); };
    view.innerHTML = '';
    view.appendChild(page);
  };

  const details = $('#details');
  let detailsState = null;

  function closeDetails() {
    details.hidden = true;
    details.innerHTML = '';
    detailsState = null;
  }

  async function openDetails(kind, id, opts) {
    opts = opts || {};
    details.hidden = false;
    details.innerHTML = '<div class="dl-wrap"><div class="dl-head skeleton" style="border-radius:0"></div><div class="dl-body"><div><div class="skeleton" style="height:18px;width:60%"></div></div></div></div>';
    const key = `${kind}-${id}`;
    detailsState = { key };
    let d, list = { episodes: [], dub: false };
    try {
      const jobs = [api.get('details', { kind, id: String(id) })];
      if (kind !== 'movie') jobs.push(api.get('title', { kind, id: String(id), dub: S.dubPreferred && kind === 'anime' ? '1' : '0' }).catch(() => list));
      jobs.push(loadHistory(false));
      const out = await Promise.all(jobs);
      d = out[0];
      if (kind !== 'movie') list = out[1] || list;
    } catch (e) {
      details.innerHTML = `<div class="dl-wrap"><button class="dl-close">${ic('x')}</button><div class="empty" style="padding:120px 0"><b>Couldn't load this title</b>${esc(e)}</div></div>`;
      $('.dl-close', details).onclick = closeDetails;
      return;
    }
    if (!detailsState || detailsState.key !== key) return;
    const title = { id: d.id, kind: d.kind, name: d.name, year: d.year, rating: d.rating, overview: d.overview, poster: d.poster, backdrop: d.backdrop, genres: d.genres, seasons: d.seasons };
    const row = opts.row || rowFor(kind, id);
    const state = Object.assign(detailsState, { d, title, list, row, season: null, watched: Memory.watched(key), inList: false, downloaded: new Set((App.downloads || []).filter(x => x.state === 'done').map(x => x.key)) });
    api.get('downloads').then(rows => { App.downloads = rows; state.downloaded = new Set(rows.filter(x => x.state === 'done').map(x => x.key)); if (detailsState === state) renderEpisodes(); }).catch(() => {});
    if (App.watchlist) state.inList = App.watchlist.some(w => w.kind === kind && w.id === id);
    else api.get('watchlist').then(rows => { App.watchlist = rows; state.inList = rows.some(w => w.kind === kind && w.id === id); const b = $('#d-list', details); if (b) b.classList.toggle('on', state.inList); }).catch(() => {});
    renderDetails();
    if (opts.autoplay) { const item = resumeItem(state); if (item) Player.start(item); }
  }

  function rowFor(kind, id) {
    const hist = (App.history || []).find(r => r.kind === kind && r.id === id);
    if (hist) return hist;
    const local = Memory.lastPlayed({ kind, id });
    if (local) return { id, kind, show: local.title, season: local.season, episode: local.episode, position: local.position, duration: local.duration };
    return null;
  }

  function episodeIndex(state, row) {
    if (!row) return -1;
    return state.list.episodes.findIndex(e => (state.title.kind === 'anime' ? Math.trunc(e.number) === row.episode : e.season === row.season && Math.trunc(e.number) === row.episode));
  }

  function resumeItem(state) {
    const { title, list, row, d } = state;
    const base = title.kind === 'movie' ? [] : list.episodes;
    if (title.kind === 'movie') {
      const start = row && row.duration > 0 && row.duration - row.position > 30 ? row.position : null;
      return Item.make(title, [], 0, false, start, d.seasons || []);
    }
    if (!base.length) return null;
    let idx = episodeIndex(state, row);
    let start = null;
    if (idx >= 0) {
      const finished = row.duration > 0 && row.duration - row.position < 30;
      if (finished) idx = Math.min(idx + 1, base.length - 1);
      else if (row.position > 0) start = row.position;
    } else {
      idx = 0;
      const seen = state.watched;
      const firstUnseen = base.findIndex(e => !seen.has(`${e.season}-${Math.trunc(e.number)}`));
      if (firstUnseen > 0) idx = firstUnseen;
    }
    return Item.make(title, base, idx, list.dub, start, d.seasons || []);
  }

  function playLabel(state) {
    const item = resumeItem(state);
    if (!item) return { text: 'Play', item: null };
    if (item.title.kind === 'movie') return { text: item.startAt ? `Resume · ${clock(item.startAt)}` : 'Play', item };
    const e = Item.ep(item);
    const tag = e ? epTag(e, item.title.kind === 'anime') : '';
    return { text: `${item.startAt ? 'Resume' : 'Play'} ${tag}`, item };
  }

  function renderDetails() {
    const state = detailsState;
    const { d, title, list } = state;
    const play = playLabel(state);
    const wrap = el(`<div class="dl-wrap">
      <button class="dl-close" aria-label="Close">${ic('x')}</button>
      <div class="dl-head"><div class="bd" style="background-image:url('${esc(d.backdrop || d.poster || '')}')"></div>
        <div class="copy">${d.logo ? `<img class="logo" src="${esc(d.logo)}" alt="">` : `<div class="name">${esc(d.name)}</div>`}${meta(d)}
          <div class="actions">
            <button class="btn primary" id="d-play">${ic('play')} ${esc(play.text)}</button>
            <button class="btn round ${state.inList ? 'on' : ''}" id="d-list" title="My List">${ic(state.inList ? 'check' : 'plus')}</button>
            ${title.kind === 'movie' ? `<button class="btn round" id="d-dl" title="Download">${ic(state.downloaded.has(`movie-${title.id}`) ? 'check' : 'download')}</button>` : `<button class="btn round" id="d-more" title="More">${ic('list')}</button>`}
            <button class="btn round" id="d-share" title="Copy link">${ic('share')}</button>
          </div></div></div>
      <div class="dl-body"><div class="main">
        ${d.tagline ? `<div class="tagline">${esc(d.tagline)}</div>` : ''}
        <div class="over">${esc(d.overview || '')}</div>
        <div class="episodes"></div>
      </div><div class="side kv">
        ${d.cast && d.cast.length ? `<div><b>Cast</b><br>${d.cast.slice(0, 8).map(esc).join(', ')}</div><br>` : ''}
        ${d.genres && d.genres.length ? `<div><b>Genres</b><br>${d.genres.map(esc).join(', ')}</div><br>` : ''}
        ${d.runtime ? `<div><b>Runtime</b><br>${human(d.runtime * 60)}</div><br>` : ''}
        ${state.row && state.row.duration ? `<div><b>Last watched</b><br>${esc(state.row.kind === 'movie' ? clock(state.row.position) : epTag({ season: state.row.season, number: state.row.episode }, state.row.kind === 'anime') + ' · ' + clock(state.row.position))}</div>` : ''}
      </div></div></div>`);
    $('.dl-close', wrap).onclick = closeDetails;
    $('#d-play', wrap).onclick = () => { const p = playLabel(state); if (p.item) Player.start(p.item); else toast('No episodes listed yet'); };
    $('#d-list', wrap).onclick = async e => {
      const b = e.currentTarget;
      try {
        if (state.inList) { await api.post('watchlist/remove', { kind: title.kind, id: title.id }); state.inList = false; toast('Removed from My List'); }
        else { await api.post('watchlist', { kind: title.kind, id: title.id, name: title.name, poster: d.poster }); state.inList = true; toast('Added to My List'); }
        App.watchlist = null;
        b.classList.toggle('on', state.inList);
        b.innerHTML = ic(state.inList ? 'check' : 'plus');
      } catch (err) { toast(String(err)); }
    };
    $('#d-share', wrap).onclick = () => { const url = `${App.boot.host || 'https://viel.tv'}/${title.kind}/${title.id}`; navigator.clipboard.writeText(url).then(() => toast('Link copied')); };
    const dl = $('#d-dl', wrap);
    if (dl) dl.onclick = () => startDownload(Item.make(title, [], 0, false, null, []), dl);
    const more = $('#d-more', wrap);
    if (more) more.onclick = e => menuAt(e.currentTarget, [
      { label: 'Download this season', icon: 'download', run: () => downloadSeason(state, false) },
      { label: 'Download unwatched', icon: 'download', run: () => downloadSeason(state, true) },
      { label: 'Mark season as watched', icon: 'check', run: () => { visibleEpisodes(state).forEach(e => Memory.markWatched(`${e.season}-${Math.trunc(e.number)}`, state.key)); state.watched = Memory.watched(state.key); renderEpisodes(); } },
      title.kind === 'anime' ? { label: list.dub ? 'Switch to subbed' : 'Switch to dubbed', icon: 'audio', run: () => switchDub(state) } : null,
    ].filter(Boolean));
    details.innerHTML = '';
    details.appendChild(wrap);
    details.scrollTop = 0;
    renderEpisodes();
  }

  function visibleEpisodes(state) {
    const eps = state.list.episodes;
    if (state.title.kind === 'anime') return eps;
    const seasons = Array.from(new Set(eps.map(e => e.season)));
    if (state.season == null) {
      const idx = episodeIndex(state, state.row);
      state.season = idx >= 0 ? eps[idx].season : seasons.find(s => s > 0) ?? seasons[0];
    }
    return eps.filter(e => e.season === state.season);
  }

  function renderEpisodes() {
    const state = detailsState;
    if (!state) return;
    const box = $('.episodes', details);
    if (!box) return;
    const { title, list, d } = state;
    box.innerHTML = '';
    if (title.kind === 'movie') return;
    const eps = list.episodes;
    const chips = el('<div class="chips"></div>');
    if (title.kind === 'tv') {
      const seasons = Array.from(new Set(eps.map(e => e.season)));
      visibleEpisodes(state);
      seasons.forEach(s => { const c = el(`<button class="chip ${s === state.season ? 'on' : ''}">${s === 0 ? 'Specials' : `Season ${s}`}</button>`); c.onclick = () => { state.season = s; renderEpisodes(); }; chips.appendChild(c); });
    } else if (d.seasons && d.seasons.length > 1) {
      d.seasons.forEach(s => { const c = el(`<button class="chip ${s.id === title.id ? 'on' : ''}" title="${esc(s.year || '')}">${esc(s.title || `Season ${s.season}`)}</button>`); c.onclick = () => { if (s.id !== title.id) openDetails('anime', s.id); }; chips.appendChild(c); });
    }
    if (title.kind === 'anime') { const c = el(`<button class="chip ${list.dub ? 'on' : ''}" style="margin-left:auto">${list.dub ? 'Dub' : 'Sub'}</button>`); c.onclick = () => switchDub(state); chips.appendChild(c); }
    if (chips.children.length) box.appendChild(chips);
    const visible = visibleEpisodes(state);
    if (!visible.length) { box.appendChild(el('<div class="empty">No episodes listed.</div>')); return; }
    const cur = episodeIndex(state, state.row);
    const wrap = el('<div class="eps"></div>');
    visible.forEach(e => {
      const i = eps.indexOf(e);
      const item = Item.make(title, eps, i, list.dub, null, d.seasons || []);
      const ek = `${e.season}-${Math.trunc(e.number)}`;
      const seen = state.watched.has(ek);
      const isCur = i === cur && state.row && !(state.row.duration > 0 && state.row.duration - state.row.position < 30);
      const frac = isCur && state.row.duration ? state.row.position / state.row.duration : (seen ? 1 : 0);
      const dkey = Item.downloadKey(item);
      const have = state.downloaded.has(dkey);
      const r = el(`<div class="ep ${isCur ? 'on' : ''}"><div class="still">${img(e.still || d.backdrop)}${frac > 0 ? `<div class="prog"><b style="width:${Math.round(frac * 100)}%"></b></div>` : ''}${seen && !isCur ? `<div class="seen">${ic('check')}</div>` : ''}</div>
        <div><div class="ttl">${esc(e.name || `Episode ${numText(e.number)}`)}<small>${esc(epTag(e, title.kind === 'anime'))}${e.runtime || d.runtime ? ` · ${human((e.runtime || d.runtime) * 60)}` : ''}${isCur ? ` · ${clock(Math.max(state.row.duration - state.row.position, 0))} left` : ''}</small></div><div class="ov">${esc(e.overview || '')}</div></div>
        <button class="dl ${have ? 'done' : ''}" title="${have ? 'Downloaded' : 'Download'}">${ic(have ? 'check' : 'download')}</button></div>`);
      r.addEventListener('click', ev => {
        if (ev.target.closest('.dl')) { if (!have) startDownload(item, ev.target.closest('.dl')); return; }
        Player.start(Item.make(title, eps, i, list.dub, isCur ? state.row.position : null, d.seasons || []));
      });
      wrap.appendChild(r);
    });
    box.appendChild(wrap);
  }

  async function switchDub(state) {
    try {
      const list = await api.get('title', { kind: 'anime', id: String(state.title.id), dub: state.list.dub ? '0' : '1' });
      if (list.dub === state.list.dub) { toast(list.dub ? 'Only the dub is available' : 'No dub for this one'); return; }
      state.list = list;
      S.dubPreferred = list.dub; Settings.save();
      renderEpisodes();
    } catch (e) { toast(String(e)); }
  }

  async function startDownload(item, button) {
    const key = Item.downloadKey(item);
    if (App.active[key]) { toast('Already downloading'); return; }
    const body = Object.assign(Item.request(item), { key, meta: Item.downloadMeta(item), max_height: S.downloadHeight });
    try {
      await invoke('download_start', { body });
      App.active[key] = { key, done: 0, total: 0, state: 'resolving' };
      updateBadge();
      toast(`Downloading ${Item.heading(item)}`);
      if (button) { button.innerHTML = ic('check'); button.classList.add('done'); }
    } catch (e) { toast(String(e)); }
  }

  async function downloadSeason(state, onlyUnwatched) {
    const eps = visibleEpisodes(state);
    const picks = eps.filter(e => !onlyUnwatched || !state.watched.has(`${e.season}-${Math.trunc(e.number)}`));
    let n = 0;
    for (const e of picks) {
      const item = Item.make(state.title, state.list.episodes, state.list.episodes.indexOf(e), state.list.dub, null, []);
      const key = Item.downloadKey(item);
      if (state.downloaded.has(key) || App.active[key]) continue;
      const body = Object.assign(Item.request(item), { key, meta: Item.downloadMeta(item), max_height: S.downloadHeight });
      try { await invoke('download_start', { body }); App.active[key] = { key, done: 0, total: 0, state: 'resolving' }; n++; } catch (_) {}
      await sleep(400);
    }
    updateBadge();
    toast(n ? `Queued ${n} episode${n === 1 ? '' : 's'}` : 'Nothing new to download');
  }

  function updateBadge() {
    const n = Object.keys(App.active).length;
    const b = $('#dl-badge');
    b.hidden = !n;
    b.textContent = n;
  }

  listen('download', p => {
    if (p.state === 'done' || p.state === 'cancelled' || String(p.state).startsWith('failed')) {
      delete App.active[p.key];
      if (p.state === 'done') toast('Download finished');
      if (String(p.state).startsWith('failed')) toast(`Download failed: ${p.state.slice(8)}`);
      if (detailsState && p.state === 'done') { detailsState.downloaded.add(p.key); renderEpisodes(); }
      if (App.view === 'downloads') Views.downloads();
    } else {
      App.active[p.key] = p;
      const li = $(`.li[data-key="${p.key}"]`);
      if (li) { const bar = $('.bar b', li); if (bar && p.total) bar.style.width = `${Math.round(p.done / p.total * 100)}%`; const s = $('.s', li); if (s && p.total) s.textContent = `${(App.downloads.find(x => x.key === p.key) || { meta: {} }).meta.label || ''} · ${Math.round(p.done / p.total * 100)}% · ${p.done}/${p.total} parts`; }
      else if (App.view === 'downloads' && p.state === 'downloading' && p.done === 0) Views.downloads();
    }
    updateBadge();
  });

  const ctx = $('#ctx');
  function menuAt(anchor, items, opts) {
    opts = opts || {};
    closeMenus();
    const m = opts.into || ctx;
    m.innerHTML = (opts.title ? `<h4>${esc(opts.title)}</h4>` : '') + items.map((it, i) => it === 'hr' ? '<hr>' : `<button class="mi ${it.on ? 'on' : ''}" data-i="${i}">${it.icon ? ic(it.icon) : ''}<span>${esc(it.label)}</span>${it.toggle ? '<i class="toggle"></i>' : ''}${it.hint ? `<small>${esc(it.hint)}</small>` : ''}</button>`).join('');
    m.hidden = false;
    m.onclick = e => { const b = e.target.closest('.mi'); if (!b) return; const it = items[Number(b.dataset.i)]; if (!it.keep) closeMenus(); if (it.run) it.run(); };
    const r = anchor.getBoundingClientRect();
    if (m === ctx) {
      m.style.left = `${Math.min(r.left, window.innerWidth - m.offsetWidth - 12)}px`;
      m.style.top = `${r.bottom + 6 + m.offsetHeight > window.innerHeight ? r.top - m.offsetHeight - 6 : r.bottom + 6}px`;
    } else {
      m.style.left = `${clamp(r.left + r.width / 2 - m.offsetWidth / 2, 12, window.innerWidth - m.offsetWidth - 12)}px`;
      m.style.top = 'auto';
      m.style.bottom = `${window.innerHeight - r.top + 10}px`;
    }
  }
  function closeMenus() { ctx.hidden = true; $('#pl-menu').hidden = true; }
  document.addEventListener('mousedown', e => { if (!e.target.closest('.menu') && !e.target.closest('[data-menu]')) closeMenus(); });

  const loginBox = $('#login');
  function showLogin() { loginBox.hidden = false; $('#login-err').textContent = ''; setTimeout(() => $('#login-form input').focus(), 50); }
  $('#login-form').addEventListener('submit', e => { e.preventDefault(); doLogin(false); });
  $('#login-register').onclick = () => doLogin(true);
  async function doLogin(register) {
    const f = $('#login-form');
    const username = f.username.value.trim(), password = f.password.value;
    if (!username || !password) { $('#login-err').textContent = 'Fill in both fields.'; return; }
    try {
      const r = await api.post(register ? 'register' : 'login', { username, password });
      App.user = r.username;
      $('#nav-user').textContent = App.user;
      loginBox.hidden = true;
      App.home = null;
      setView('home');
    } catch (err) { $('#login-err').textContent = String(err); }
  }

  const Player = (() => {
    const root = $('#player');
    const ui = {
      name: $('#pl-name'), sub: $('#pl-sub'), source: $('#pl-source'), flash: $('#pl-flash'), spinner: $('#pl-spinner'), status: $('#pl-status'),
      skip: $('#pl-skip'), next: $('#pl-next'), scrub: $('#pl-scrub'), fill: $('#pl-scrub .fill'), buf: $('#pl-scrub .buf'), knob: $('#pl-scrub .knob'), hover: $('#pl-scrub .hover-time'),
      play: $('#pl-play'), pos: $('#pl-pos'), dur: $('#pl-dur'), vol: $('#pl-vol'), mute: $('#pl-mute'), speed: $('#pl-speed'), cc: $('#pl-cc'), audio: $('#pl-audio'), quality: $('#pl-quality'),
      nextbtn: $('#pl-nextbtn'), mini: $('#pl-mini'), full: $('#pl-full'), stats: $('#pl-stats'), menu: $('#pl-menu'), panel: $('#pl-panel'), episodes: $('#pl-episodes'), gear: $('#pl-gear'), back: $('#pl-back'), rew: $('#pl-rew'), ffw: $('#pl-ffw'),
    };
    const st = {
      open: false, ready: false, item: null, streams: [], streamIndex: -1, stream: null, probes: [], loading: false, pos: 0, dur: 0, paused: true, buffering: false, eof: false,
      tracks: [], sid: null, subs: [], audioTracks: null, pendingSeek: null, lastSave: 0, lastSavedPos: -1, dismissedNext: false, skipped: new Set(), hideTimer: null, flashTimer: null,
      fullscreen: false, mini: false, scrubbing: false, props: {}, sleepAt: null, sleepAtEnd: false, sleepTimer: null, statsOn: false, finishedAt: 0, started: 0,
    };
    let cmdQueue = Promise.resolve();
    const cmd = (...args) => invoke('player_cmd', { args: args.map(String) }).catch(e => log('mpv', `${args.join(' ')} → ${e}`));
    const set = (name, value) => invoke('player_set', { name, value: String(value) }).catch(e => log('mpv', `set ${name} → ${e}`));
    const get = name => invoke('player_get', { name }).catch(() => null);

    async function init() {
      try {
        await invoke('player_init');
        st.ready = true;
        await set('volume', S.volume);
        await set('mute', S.mute ? 'yes' : 'no');
        await set('hwdec', S.hwdec ? 'auto-safe' : 'no');
        applySubStyle();
      } catch (e) {
        st.ready = false;
        st.initError = String(e);
        log('mpv', `init failed: ${e}`);
      }
    }

    function applySubStyle() {
      if (!st.ready) return;
      set('sub-font-size', Math.round(48 * (S.subSize || 1)));
      set('sub-back-color', S.subBackground ? '#66000000' : '#00000000');
      set('sub-shadow-offset', S.subBackground ? 0 : 2);
      set('sub-border-size', S.subBackground ? 1.5 : 2.6);
    }

    function refreshSkip() { $$('.skipnum', root).forEach(b => b.textContent = S.skipSeconds); }

    async function start(item) {
      if (!st.ready) { toast(st.initError ? `Player unavailable: ${st.initError}` : 'Player is still starting'); return; }
      closeMenus();
      st.item = item;
      st.streams = []; st.stream = null; st.streamIndex = -1; st.probes = [];
      st.pos = item.startAt || 0; st.dur = 0; st.eof = false; st.dismissedNext = false; st.skipped = new Set(); st.subs = []; st.sid = null; st.audioTracks = null; st.lastSavedPos = -1; st.finishedAt = 0;
      st.started = Date.now();
      ui.panel.hidden = true; ui.next.hidden = true; ui.skip.hidden = true;
      if (!st.open) {
        st.open = true;
        root.hidden = false;
        document.documentElement.classList.add('video');
        invoke('player_show', { visible: true });
        window.addEventListener('keydown', onKey, true);
      }
      renderTitle();
      ui.source.textContent = '';
      status('Finding a source…');
      ui.spinner.hidden = false;
      showControls();
      updateTime();
      ui.fill.style.width = ui.buf.style.width = '0%';
      await cmd('stop');
      let reply;
      try {
        reply = item.offline ? await api.get('offline', { key: item.offline }) : await api.post('play', Item.request(item));
      } catch (e) {
        status(`Couldn't resolve this one. ${e}`, true);
        ui.spinner.hidden = true;
        return;
      }
      if (st.item !== item) return;
      let streams = reply.streams || [];
      const pref = Memory.preferredSource(item.title);
      if (pref) streams = streams.slice().sort((a, b) => (a.source === pref ? 0 : 1) - (b.source === pref ? 0 : 1));
      st.streams = streams;
      if (!streams.length) { status('No source has this one right now.', true); ui.spinner.hidden = true; return; }
      loadStream(0);
    }

    function status(text, final) {
      ui.status.hidden = !text;
      ui.status.innerHTML = text ? `<div>${esc(text)}</div>${!final && st.probes.length ? `<div class="probes">${st.probes.map(p => `<span class="pill ${p.state}">${esc(sourceLabel(p.name))}</span>`).join('')}</div>` : ''}` : '';
      if (final) ui.spinner.hidden = true;
    }

    async function loadStream(i, resumeAt) {
      const s = st.streams[i];
      if (!s) { status('Every source failed for this one. Try again in a bit.', true); return; }
      st.streamIndex = i;
      st.stream = s;
      st.loading = true;
      st.pendingSeek = resumeAt != null ? resumeAt : (st.item.startAt || null);
      st.subs = s.subs || [];
      st.sid = null;
      ui.source.textContent = sourceLabel(s.source);
      ui.spinner.hidden = false;
      status(st.streams.length > 1 && i > 0 ? `Trying ${sourceLabel(s.source)}…` : 'Loading…');
      await set('start', st.pendingSeek && st.pendingSeek > 0 ? String(st.pendingSeek) : 'none');
      await set('pause', 'no');
      await cmd('loadfile', s.url, 'replace');
      log('play', `${Item.heading(st.item)} via ${s.source}${st.pendingSeek ? ` @${Math.round(st.pendingSeek)}` : ''}`);
    }

    async function onLoaded() {
      st.loading = false;
      ui.spinner.hidden = true;
      status('');
      if (st.stream && st.stream.source !== 'offline') Memory.rememberSource(st.item.title, st.stream.source);
      await set('start', 'none');
      const want = Memory.subtitle(st.item.title) || S.subtitleLang;
      let pick = -1;
      for (let i = 0; i < st.subs.length; i++) {
        const sub = st.subs[i];
        await cmd('sub-add', sub.url, 'auto', langName(sub.lang), sub.lang);
        if (pick < 0 && want !== 'off' && langMatch(sub.lang, want)) pick = i;
      }
      if (pick < 0 && want !== 'off' && st.subs.length && langMatch(st.subs[0].lang, S.subtitleLang)) pick = 0;
      await selectSub(pick >= 0 ? pick : null, true);
      if (st.stream && st.stream.hls && st.stream.source !== 'offline') api.get('audio', { url: st.stream.url }).then(rows => { st.audioTracks = rows || []; }).catch(() => { st.audioTracks = []; });
      scheduleHide();
    }

    const langMatch = (lang, pref) => { const a = String(lang || '').toLowerCase(), b = String(pref || '').toLowerCase(); return !!b && (a === b || a.slice(0, 2) === b.slice(0, 2)); };

    async function selectSub(index, silent) {
      st.sid = index;
      if (index == null) await set('sid', 'no');
      else {
        let tracks = [];
        try { tracks = JSON.parse(await get('track-list')) || []; } catch (_) {}
        const external = tracks.filter(t => t.type === 'sub' && t.external);
        const track = external[index];
        await set('sid', track ? track.id : index + 1);
        await set('sub-visibility', 'yes');
      }
      ui.cc.classList.toggle('on', index != null);
      if (!silent) { Memory.rememberSubtitle(st.item.title, index == null ? null : st.subs[index].lang); flash(index == null ? 'Subtitles off' : langName(st.subs[index].lang), true); }
    }

    function renderTitle() {
      const it = st.item;
      ui.name.textContent = it.title.name;
      ui.sub.textContent = Item.ep(it) ? epLabel(Item.ep(it), Item.isAnime(it)) + (it.dub ? ' · Dub' : '') : (it.title.year || '');
      ui.nextbtn.hidden = !Item.next(it);
      ui.episodes.hidden = Item.isMovie(it) || !it.episodes.length;
    }

    function updateTime() {
      ui.pos.textContent = clock(st.pos);
      ui.dur.textContent = clock(st.dur);
      if (!st.scrubbing) {
        const f = st.dur ? st.pos / st.dur : 0;
        ui.fill.style.width = `${f * 100}%`;
        ui.knob.style.left = `${f * 100}%`;
      }
      const cache = st.props['demuxer-cache-duration'] || 0;
      ui.buf.style.width = st.dur ? `${clamp((st.pos + cache) / st.dur, 0, 1) * 100}%` : '0%';
    }

    function tick() {
      if (!st.open || !st.item) return;
      if (st.dur > 0 && !st.loading && st.pos > 0 && Date.now() - st.lastSave > 5000 && Math.abs(st.pos - st.lastSavedPos) > 3) save(false);
      checkMarkers();
      checkNext();
      if (st.statsOn) renderStats();
      if (st.sleepAt && Date.now() >= st.sleepAt) { st.sleepAt = null; close(); }
    }

    function save(final) {
      if (!st.item || !(st.dur > 0) || st.item.offline) return;
      st.lastSave = Date.now();
      st.lastSavedPos = st.pos;
      const p = Item.progress(st.item, final ? st.dur : st.pos, st.dur);
      Memory.remember(p);
      api.post('progress', p).catch(e => log('progress', String(e)));
    }

    function checkMarkers() {
      const s = st.stream;
      if (!s) { ui.skip.hidden = true; return; }
      let which = null;
      for (const k of ['intro', 'outro']) {
        const m = s[k];
        if (m && m.length === 2 && !st.skipped.has(k) && st.pos >= m[0] && st.pos < m[1] - 1) { which = k; break; }
      }
      if (!which) { ui.skip.hidden = true; return; }
      if (ui.skip.hidden || ui.skip.dataset.which !== which) {
        ui.skip.dataset.which = which;
        ui.skip.innerHTML = `${which === 'intro' ? 'Skip Intro' : 'Skip Outro'} ${ic('next')}`;
        ui.skip.hidden = false;
        ui.skip.onclick = () => { st.skipped.add(which); seek(s[which][1], true); ui.skip.hidden = true; };
      }
    }

    function checkNext() {
      const next = st.item && Item.next(st.item);
      const show = next && S.autoplay && !st.dismissedNext && st.dur > 0 && !st.loading && st.dur - st.pos <= 20 && st.dur - st.pos > 0;
      if (!show) { ui.next.hidden = true; return; }
      const left = Math.ceil(st.dur - st.pos);
      const e = Item.ep(next);
      if (ui.next.hidden) {
        ui.next.innerHTML = `<div class="still">${img(e.still || next.title.poster)}<div class="count"></div></div><div><div class="k">Up next</div><div class="n">${esc(epLabel(e, Item.isAnime(next)))}</div><div class="btns"><button class="btn sm primary" id="nx-play">${ic('play')} Play now</button><button class="btn sm ghost" id="nx-x">Cancel</button></div></div>`;
        ui.next.hidden = false;
        $('#nx-play', ui.next).onclick = ev => { ev.stopPropagation(); goNext(); };
        $('#nx-x', ui.next).onclick = ev => { ev.stopPropagation(); st.dismissedNext = true; ui.next.hidden = true; };
        ui.next.onclick = () => goNext();
      }
      $('.count', ui.next).textContent = left;
    }

    function goNext() {
      const next = st.item && Item.next(st.item);
      if (!next) return;
      save(true);
      api.post('progress', Item.progress(next, 0, st.dur || 0)).catch(() => {});
      ui.next.hidden = true;
      start(next);
    }

    function finished() {
      if (st.finishedAt && Date.now() - st.finishedAt < 2000) return;
      st.finishedAt = Date.now();
      save(true);
      const next = Item.next(st.item);
      if (next && !st.item.offline) api.post('progress', Item.progress(next, 0, st.dur || 0)).catch(() => {});
      if (st.sleepAtEnd) { close(); return; }
      if (next && S.autoplay && !st.dismissedNext && !st.item.offline) start(next);
      else close();
    }

    async function close() {
      if (!st.open) return;
      save(false);
      st.open = false;
      clearTimeout(st.sleepTimer);
      st.sleepAt = null; st.sleepAtEnd = false;
      window.removeEventListener('keydown', onKey, true);
      await cmd('stop');
      invoke('player_show', { visible: false });
      if (st.fullscreen) { st.fullscreen = false; invoke('win', { action: 'fullscreen', on: false }); }
      if (st.mini) toggleMini(false);
      root.hidden = true;
      ui.status.hidden = true; ui.spinner.hidden = true; ui.next.hidden = true; ui.skip.hidden = true; ui.panel.hidden = true; ui.stats.hidden = true;
      document.documentElement.classList.remove('video');
      closeMenus();
      App.homeAt = 0; App.historyAt = 0;
      if (detailsState) { const row = rowFor(detailsState.title.kind, detailsState.title.id); detailsState.row = row; detailsState.watched = Memory.watched(detailsState.key); renderDetails(); }
      if (App.view === 'home') Views.home();
      if (App.view === 'history') Views.history();
    }

    function flash(content, text) {
      ui.flash.classList.toggle('text', !!text);
      ui.flash.innerHTML = text ? esc(content) : ic(content);
      ui.flash.classList.add('show');
      clearTimeout(st.flashTimer);
      st.flashTimer = setTimeout(() => ui.flash.classList.remove('show'), text ? 900 : 500);
    }

    function togglePlay() {
      if (!st.item || st.loading) return;
      const toPause = !st.paused;
      set('pause', toPause ? 'yes' : 'no');
      flash(toPause ? 'pause' : 'play');
      if (toPause) showControls(); else scheduleHide();
    }

    function seek(to, absolute) {
      if (!st.dur) return;
      const target = clamp(absolute ? to : st.pos + to, 0, Math.max(st.dur - 0.5, 0));
      st.pos = target;
      updateTime();
      cmd('seek', target, 'absolute+exact');
      if (!absolute) flash(`${to > 0 ? '+' : '−'}${Math.abs(to)}s`, true);
      showControls();
    }

    function setVolume(v, quiet) {
      v = clamp(Math.round(v), 0, 150);
      Settings.set('volume', v);
      set('volume', v);
      ui.vol.value = Math.min(v, 100);
      if (S.mute && v > 0) { Settings.set('mute', false); set('mute', 'no'); }
      ui.mute.innerHTML = ic(S.mute || v === 0 ? 'volx' : 'vol');
      if (!quiet) flash(`Volume ${v}%`, true);
    }

    function toggleMute() {
      Settings.set('mute', !S.mute);
      set('mute', S.mute ? 'yes' : 'no');
      ui.mute.innerHTML = ic(S.mute ? 'volx' : 'vol');
      flash(S.mute ? 'Muted' : `Volume ${S.volume}%`, true);
    }

    function setSpeed(v) {
      v = clamp(Math.round(v * 100) / 100, 0.25, 4);
      Settings.set('speed', v);
      set('speed', v);
      ui.speed.textContent = `${v}×`;
      flash(`${v}× speed`, true);
    }

    function toggleFullscreen(on) {
      st.fullscreen = on == null ? !st.fullscreen : on;
      invoke('win', { action: 'fullscreen', on: st.fullscreen });
      ui.full.innerHTML = ic(st.fullscreen ? 'unfull' : 'full');
    }

    function toggleMini(on) {
      st.mini = on == null ? !st.mini : on;
      invoke('win', { action: 'mini', on: st.mini });
      root.classList.toggle('mini', st.mini);
      ui.mini.classList.toggle('on', st.mini);
    }

    function showControls() {
      root.classList.remove('hide-ui');
      clearTimeout(st.hideTimer);
      if (!st.paused && !st.loading) scheduleHide();
    }
    function scheduleHide() {
      clearTimeout(st.hideTimer);
      st.hideTimer = setTimeout(() => { if (!st.paused && !st.scrubbing && ui.menu.hidden && ui.panel.hidden) root.classList.add('hide-ui'); }, 3200);
    }

    function renderStats() {
      const p = st.props;
      const w = p['video-params/w'], h = p['video-params/h'];
      const lines = [
        ['Source', `${sourceLabel(st.stream && st.stream.source)}${st.stream && st.stream.hls ? ' · HLS' : ''}`],
        ['Video', `${w && h ? `${w}×${h}` : '—'} ${p['video-codec'] ? `· ${String(p['video-codec']).split(' ')[0]}` : ''}`],
        ['Audio', p['audio-codec-name'] || '—'],
        ['Decoder', p['hwdec-current'] && p['hwdec-current'] !== 'no' ? `${p['hwdec-current']} (GPU)` : 'software'],
        ['FPS', p['estimated-vf-fps'] ? `${Number(p['estimated-vf-fps']).toFixed(2)} / ${Number(p['container-fps'] || 0).toFixed(2)}` : '—'],
        ['Bitrate', p['video-bitrate'] ? `${(p['video-bitrate'] / 1e6).toFixed(1)} Mbit/s` : '—'],
        ['Buffer', `${Number(p['demuxer-cache-duration'] || 0).toFixed(1)} s ahead · ${p['cache-speed'] ? `${(p['cache-speed'] / 125000).toFixed(1)} Mbit/s` : '—'}`],
        ['Dropped', String(p['frame-drop-count'] || 0)],
        ['Speed', `${p.speed || 1}×`],
        ['Relay', `127.0.0.1:${App.boot.port}`],
      ];
      ui.stats.textContent = lines.map(([k, v]) => `${k.padEnd(9)}${v}`).join('\n');
    }

    function onProp(name, value) {
      st.props[name] = value;
      switch (name) {
        case 'pause': st.paused = !!value; ui.play.innerHTML = ic(st.paused ? 'play' : 'pause'); if (st.paused) showControls(); else scheduleHide(); break;
        case 'time-pos': if (value != null && !st.scrubbing) { st.pos = value; updateTime(); } break;
        case 'duration': if (value != null) { st.dur = value; updateTime(); } break;
        case 'paused-for-cache': st.buffering = !!value; ui.spinner.hidden = !(value || st.loading); break;
        case 'eof-reached': if (value && st.dur > 0 && !st.loading) finished(); break;
        case 'speed': if (value) ui.speed.textContent = `${Math.round(value * 100) / 100}×`; break;
        case 'volume': if (value != null && !st.scrubbing) ui.vol.value = Math.min(Math.round(value), 100); break;
        case 'demuxer-cache-duration': updateTime(); break;
      }
    }

    function onEvent(p) {
      switch (p.event) {
        case 'loaded': onLoaded(); break;
        case 'start': break;
        case 'end':
          if (!st.open) break;
          if (p.reason === 'error') {
            log('play', `${st.stream ? st.stream.source : '?'} failed: ${p.error}`);
            if (st.streams[st.streamIndex + 1]) { toast(`${sourceLabel(st.stream.source)} failed, trying ${sourceLabel(st.streams[st.streamIndex + 1].source)}`); loadStream(st.streamIndex + 1, st.pos > 5 ? st.pos : null); }
            else status(`Couldn't play this one (${p.error}).`, true);
          } else if (p.reason === 'eof' && st.dur > 0 && !st.loading) finished();
          break;
        case 'log': if (p.level === 'error' || p.level === 'fatal') log('mpv', `${p.prefix}: ${p.text}`); break;
        case 'prop': onProp(p.name, p.value); break;
      }
    }

    function episodesPanel() {
      if (!ui.panel.hidden) { ui.panel.hidden = true; return; }
      const it = st.item;
      if (!it || !it.episodes.length) return;
      const anime = Item.isAnime(it);
      const seasons = anime ? [] : Array.from(new Set(it.episodes.map(e => e.season)));
      const cur = Item.ep(it);
      let season = cur ? cur.season : seasons[0];
      const render = () => {
        ui.panel.innerHTML = `<h3>Episodes</h3>${seasons.length > 1 ? `<div class="chips">${seasons.map(s => `<button class="chip ${s === season ? 'on' : ''}" data-s="${s}">${s === 0 ? 'Specials' : `Season ${s}`}</button>`).join('')}</div>` : ''}<div class="eps"></div>`;
        const list = $('.eps', ui.panel);
        it.episodes.forEach((e, i) => {
          if (!anime && e.season !== season) return;
          const r = el(`<div class="ep ${i === it.index ? 'on' : ''}"><div class="still">${img(e.still || it.title.poster)}</div><div><div class="ttl">${esc(e.name || `Episode ${numText(e.number)}`)}</div><small class="muted">${esc(epTag(e, anime))}</small></div></div>`);
          r.onclick = () => { ui.panel.hidden = true; if (i !== it.index) start(Item.at(it, i)); };
          list.appendChild(r);
        });
        $$('.chip', ui.panel).forEach(c => c.onclick = () => { season = Number(c.dataset.s); render(); });
        const on = $('.ep.on', ui.panel);
        if (on) on.scrollIntoView({ block: 'center' });
      };
      render();
      ui.panel.hidden = false;
      showControls();
    }

    function gearMenu(anchor) {
      const sleepLabel = st.sleepAtEnd ? 'End of episode' : st.sleepAt ? `${Math.max(1, Math.round((st.sleepAt - Date.now()) / 60000))} min left` : 'Off';
      menuAt(anchor, [
        { label: 'Stats for nerds', icon: 'stats', toggle: true, on: st.statsOn, run: () => { st.statsOn = !st.statsOn; ui.stats.hidden = !st.statsOn; if (st.statsOn) renderStats(); } },
        { label: 'Autoplay next', icon: 'next', toggle: true, on: S.autoplay, run: () => Settings.set('autoplay', !S.autoplay) },
        'hr',
        { label: 'Sleep timer', icon: 'moon', hint: sleepLabel, run: () => setTimeout(() => sleepMenu(anchor), 10) },
        { label: 'Subtitle size', icon: 'cc', hint: S.subSize === 1 ? 'Normal' : S.subSize < 1 ? 'Small' : S.subSize < 1.4 ? 'Large' : 'Huge', run: () => { const steps = [0.8, 1, 1.2, 1.5]; Settings.set('subSize', steps[(steps.indexOf(S.subSize) + 1) % steps.length]); applySubStyle(); flash(`Subtitles ${S.subSize === 1 ? 'normal' : S.subSize < 1 ? 'small' : S.subSize < 1.4 ? 'large' : 'huge'}`, true); } },
        st.item && !st.item.offline ? { label: Item.isMovie(st.item) ? 'Download this movie' : 'Download this episode', icon: 'download', run: () => startDownload(st.item) } : null,
        { label: 'Copy link', icon: 'share', run: () => { const it = st.item, e = Item.ep(it); const url = `${App.boot.host}/${it.title.kind}/${it.title.id}${e ? `/${e.season}/${Math.trunc(e.number)}` : ''}`; navigator.clipboard.writeText(url).then(() => toast('Link copied')); } },
      ].filter(Boolean), { into: ui.menu });
    }

    function sleepMenu(anchor) {
      const pick = (min, atEnd) => { clearTimeout(st.sleepTimer); st.sleepAtEnd = !!atEnd; st.sleepAt = min ? Date.now() + min * 60000 : null; flash(atEnd ? 'Stops after this episode' : min ? `Sleep in ${min} min` : 'Sleep timer off', true); };
      menuAt(anchor, [
        { label: 'Off', on: !st.sleepAt && !st.sleepAtEnd, run: () => pick(0) },
        ...[15, 30, 45, 60, 90].map(m => ({ label: `${m} minutes`, run: () => pick(m) })),
        { label: 'End of this episode', on: st.sleepAtEnd, run: () => pick(0, true) },
      ], { into: ui.menu, title: 'Sleep timer' });
    }

    function subMenu(anchor) {
      const items = [{ label: 'Off', on: st.sid == null, run: () => selectSub(null) }];
      st.subs.forEach((s, i) => items.push({ label: langName(s.lang), hint: s.lang, on: st.sid === i, run: () => selectSub(i) }));
      const internal = (st.props['track-list'] || []).filter(t => t.type === 'sub' && !t.external);
      internal.forEach(t => items.push({ label: t.title || langName(t.lang) || `Track ${t.id}`, hint: 'embedded', on: t.selected, run: () => { set('sid', t.id); set('sub-visibility', 'yes'); st.sid = -1; ui.cc.classList.add('on'); } }));
      if (items.length === 1) items.push({ label: 'No subtitles for this source', run: () => {} });
      menuAt(anchor, items, { into: ui.menu, title: 'Subtitles' });
    }

    function audioMenu(anchor) {
      const items = [];
      const internal = (st.props['track-list'] || []).filter(t => t.type === 'audio');
      if (internal.length > 1) internal.forEach(t => items.push({ label: t.title || langName(t.lang) || `Track ${t.id}`, hint: t.lang || '', on: t.selected, run: () => { set('aid', t.id); flash(t.title || langName(t.lang), true); } }));
      else if (st.audioTracks && st.audioTracks.length) {
        st.audioTracks.forEach(t => items.push({ label: t.name || langName(t.lang), hint: t.lang, on: S.audioLang ? langMatch(t.lang, S.audioLang) : !!t.default, run: async () => { Settings.set('audioLang', t.lang); await Settings.push(); loadStream(st.streamIndex, st.pos); flash(`${t.name || langName(t.lang)} audio`, true); } }));
      }
      if (!items.length) items.push({ label: 'Only one audio track', run: () => {} });
      menuAt(anchor, items, { into: ui.menu, title: 'Audio' });
    }

    function qualityMenu(anchor) {
      const items = [2160, 1080, 720, 480].map(h => ({ label: h === 2160 ? '4K' : `${h}p`, hint: h === 1080 ? 'Full HD' : h === 2160 ? 'Ultra HD' : '', on: S.maxHeight === h, run: async () => { Settings.set('maxHeight', h); await Settings.push(); loadStream(st.streamIndex, st.pos); flash(h === 2160 ? '4K' : `${h}p`, true); } }));
      items.push('hr');
      st.streams.forEach((s, i) => items.push({ label: sourceLabel(s.source), hint: i === st.streamIndex ? 'playing' : 'source', on: i === st.streamIndex, run: () => { if (i !== st.streamIndex) { Memory.rememberSource(st.item.title, s.source); loadStream(i, st.pos); } } }));
      menuAt(anchor, items, { into: ui.menu, title: 'Quality' });
    }

    function onKey(e) {
      if (!st.open) return;
      if (e.target && /INPUT|TEXTAREA|SELECT/.test(e.target.tagName)) return;
      const k = e.key;
      let handled = true;
      switch (k) {
        case ' ': case 'k': case 'K': togglePlay(); break;
        case 'ArrowLeft': seek(e.shiftKey ? -60 : -S.skipSeconds); break;
        case 'ArrowRight': seek(e.shiftKey ? 60 : S.skipSeconds); break;
        case 'j': case 'J': seek(-10); break;
        case 'l': case 'L': seek(10); break;
        case 'ArrowUp': setVolume(S.volume + 5); break;
        case 'ArrowDown': setVolume(S.volume - 5); break;
        case 'm': case 'M': toggleMute(); break;
        case 'f': case 'F': toggleFullscreen(); break;
        case 'c': case 'C': if (st.subs.length) selectSub(st.sid == null ? 0 : null); break;
        case 'n': case 'N': if (Item.next(st.item)) goNext(); break;
        case 'e': case 'E': episodesPanel(); break;
        case 'p': case 'P': toggleMini(); break;
        case 'i': case 'I': st.statsOn = !st.statsOn; ui.stats.hidden = !st.statsOn; if (st.statsOn) renderStats(); break;
        case '[': setSpeed(S.speed - 0.25); break;
        case ']': setSpeed(S.speed + 0.25); break;
        case 'Backspace': setSpeed(1); break;
        case 'Escape':
          if (!ui.menu.hidden || !ui.panel.hidden) { closeMenus(); ui.panel.hidden = true; }
          else if (st.fullscreen) toggleFullscreen(false);
          else if (st.mini) toggleMini(false);
          else close();
          break;
        case 'Home': seek(0, true); break;
        case 'End': if (st.dur) seek(st.dur - 1, true); break;
        default:
          if (/^[0-9]$/.test(k) && st.dur) seek(st.dur * Number(k) / 10, true);
          else handled = false;
      }
      if (handled) { e.preventDefault(); e.stopPropagation(); showControls(); }
    }

    let clickTimer;
    root.addEventListener('mousemove', () => { if (st.open) showControls(); });
    root.addEventListener('mouseleave', () => { if (st.open && !st.paused) scheduleHide(); });
    root.addEventListener('click', e => {
      if (e.target.closest('.pl-bar, #pl-skip, #pl-next, .menu, #pl-panel, #pl-stats')) return;
      if (e.detail === 2) { clearTimeout(clickTimer); toggleFullscreen(); return; }
      clearTimeout(clickTimer);
      clickTimer = setTimeout(() => togglePlay(), 230);
    });
    root.addEventListener('wheel', e => { if (e.target.closest('.menu, #pl-panel')) return; setVolume(S.volume + (e.deltaY < 0 ? 5 : -5)); }, { passive: true });
    ui.play.onclick = togglePlay;
    ui.rew.onclick = () => seek(-S.skipSeconds);
    ui.ffw.onclick = () => seek(S.skipSeconds);
    ui.mute.onclick = toggleMute;
    ui.vol.oninput = () => setVolume(Number(ui.vol.value), true);
    ui.back.onclick = () => close();
    ui.full.onclick = () => toggleFullscreen();
    ui.mini.onclick = () => toggleMini();
    ui.nextbtn.onclick = goNext;
    ui.episodes.onclick = episodesPanel;
    ui.gear.onclick = e => gearMenu(e.currentTarget);
    ui.cc.onclick = e => subMenu(e.currentTarget);
    ui.audio.onclick = e => audioMenu(e.currentTarget);
    ui.quality.onclick = e => qualityMenu(e.currentTarget);
    ui.speed.onclick = e => menuAt(e.currentTarget, [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2].map(v => ({ label: `${v}×`, on: S.speed === v, run: () => setSpeed(v) })), { into: ui.menu, title: 'Speed' });
    [ui.gear, ui.cc, ui.audio, ui.quality, ui.speed].forEach(b => b.setAttribute('data-menu', ''));

    const scrubAt = e => { const r = ui.scrub.getBoundingClientRect(); return clamp((e.clientX - r.left) / r.width, 0, 1); };
    ui.scrub.addEventListener('mousemove', e => { if (!st.dur) return; const f = scrubAt(e); ui.hover.hidden = false; ui.hover.style.left = `${f * 100}%`; ui.hover.textContent = clock(f * st.dur); if (st.scrubbing) { ui.fill.style.width = `${f * 100}%`; ui.knob.style.left = `${f * 100}%`; ui.pos.textContent = clock(f * st.dur); } });
    ui.scrub.addEventListener('mouseleave', () => { ui.hover.hidden = true; });
    ui.scrub.addEventListener('mousedown', e => {
      if (!st.dur) return;
      st.scrubbing = true;
      ui.scrub.classList.add('drag');
      const move = ev => { const f = scrubAt(ev); ui.fill.style.width = `${f * 100}%`; ui.knob.style.left = `${f * 100}%`; ui.pos.textContent = clock(f * st.dur); };
      const up = ev => { window.removeEventListener('mousemove', move); window.removeEventListener('mouseup', up); st.scrubbing = false; ui.scrub.classList.remove('drag'); seek(scrubAt(ev) * st.dur, true); };
      window.addEventListener('mousemove', move);
      window.addEventListener('mouseup', up);
      move(e);
    });

    listen('mpv', onEvent);
    setInterval(tick, 1000);
    refreshSkip();
    ui.speed.textContent = `${S.speed || 1}×`;
    ui.vol.value = Math.min(S.volume, 100);
    ui.mute.innerHTML = ic(S.mute ? 'volx' : 'vol');

    return { init, start, close, set, applySubStyle, refreshSkip, get open() { return st.open; } };
  })();

  document.addEventListener('keydown', e => {
    if (Player.open) return;
    if (e.target && /INPUT|TEXTAREA|SELECT/.test(e.target.tagName)) { if (e.key === 'Escape') e.target.blur(); return; }
    if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K')) { e.preventDefault(); setView('search'); return; }
    if (e.key === 'Escape') { if (!ctx.hidden) closeMenus(); else if (!details.hidden) closeDetails(); return; }
    if (e.key === '/' && details.hidden) { e.preventDefault(); setView('search'); }
  });
  details.addEventListener('mousedown', e => { if (e.target === details) closeDetails(); });

  async function main() {
    hydrateIcons(document);
    try { App.boot = await invoke('boot'); } catch (e) { App.boot = { backdrop: 'none', host: 'https://viel.tv' }; }
    if (App.boot.backdrop && App.boot.backdrop !== 'none') document.documentElement.classList.add('backdrop');
    App.user = App.boot.username || null;
    if (App.user) $('#nav-user').textContent = App.user;
    Settings.push();
    Player.init();
    invoke('downloads_active').then(rows => { rows.forEach(r => { App.active[r.key] = r; }); updateBadge(); }).catch(() => {});
    if (!App.boot.token) { showLogin(); return; }
    api.get('me').then(r => { if (r && r.username) { App.user = r.username; $('#nav-user').textContent = App.user; } else if (r && r.username === null) showLogin(); }).catch(() => {});
    setView('home');
  }

  main();
})();
