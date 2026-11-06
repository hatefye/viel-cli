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
  const byId = id => document.getElementById(id);
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
  const human = min => { min = Math.round(min || 0); const h = Math.floor(min / 60), m = min % 60; return h ? `${h}h ${m}m` : `${m}m`; };
  const size = b => b >= 1073741824 ? `${(b / 1073741824).toFixed(1)} GB` : `${Math.round(b / 1048576)} MB`;

  const I = {
    play: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>',
    pause: '<svg viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="5" width="4" height="14" rx="1.2"/><rect x="14" y="5" width="4" height="14" rx="1.2"/></svg>',
    info: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M12 16v-4M12 8h.01"/></svg>',
    plus: '<svg class="plus" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>',
    tick: '<svg class="tick" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5L20 7"/></svg>',
    x: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M18 6 6 18M6 6l12 12"/></svg>',
    bookmark: '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 4h12a1 1 0 0 1 1 1v15.4a.6.6 0 0 1-.92.5L12 17.5l-6.08 3.9A.6.6 0 0 1 5 20.9V5a1 1 0 0 1 1-1z"/></svg>',
    download: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12m0 0l-4-4m4 4l4-4M5 21h14"/></svg>',
    trash: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v6M14 11v6"/></svg>',
    left: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 6l-6 6 6 6"/></svg>',
    right: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 6l6 6-6 6"/></svg>',
    star: '<svg viewBox="0 0 24 24"><path d="M12 2.6l2.9 6 6.6.9-4.8 4.6 1.2 6.5L12 17.5l-5.9 3.1 1.2-6.5L2.5 9.5l6.6-.9z"/></svg>',
    tv: '<svg viewBox="0 0 24 24"><rect x="3" y="6" width="18" height="12" rx="2.5"/><path d="M8 21h8M12 18v3M8 2l4 4 4-4"/></svg>',
    film: '<svg viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="M7 4v16M17 4v16M3 9h4M17 9h4M3 15h4M17 15h4"/></svg>',
    share: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 12v7a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-7M12 15V3M7 8l5-5 5 5"/></svg>',
    folder: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h6l2 2h10v11H3z"/></svg>',
    more: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M5 12h.01M12 12h.01M19 12h.01"/></svg>',
    next: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M6 5l8 7-8 7V5zm10 0h2v14h-2V5z"/></svg>',
    full: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/></svg>',
    unfull: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 4v5H4M20 9h-5V4M15 20v-5h5M4 15h5v5"/></svg>',
    vol: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M3 10v4h4l5 4V6L7 10H3zm13.5 2c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06c4.01-.91 7-4.49 7-8.77s-2.99-7.86-7-8.77z"/></svg>',
    volx: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M16.5 12A4.5 4.5 0 0 0 14 7.97v2.21l2.45 2.45c.03-.2.05-.41.05-.63zm2.5 0c0 .94-.2 1.82-.54 2.64l1.51 1.51A8.8 8.8 0 0 0 21 12c0-4.28-2.99-7.86-7-8.77v2.06c2.89.86 5 3.54 5 6.71zM4.27 3 3 4.27 7.73 9H3v6h4l5 4v-6.73l4.25 4.25c-.67.52-1.42.93-2.25 1.18v2.06a8.99 8.99 0 0 0 3.69-1.81L19.73 21 21 19.73 4.27 3zM12 4 9.91 6.09 12 8.18V4z"/></svg>',
    check: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5L20 7"/></svg>',
  };

  let toastTimer;
  const toast = (text, ms, onClick) => { const t = byId('toast'); t.textContent = text; t.onclick = onClick || null; t.style.cursor = onClick ? 'pointer' : ''; t.classList.add('show'); clearTimeout(toastTimer); toastTimer = setTimeout(() => t.classList.remove('show'), ms || 2600); };

  const Settings = {
    data: Object.assign({ maxHeight: 1080, downloadHeight: 1080, subtitleLang: 'en', audioLang: '', subSize: 1, skipSeconds: 10, autoplay: true, hwdec: true, volume: 100, mute: false, speed: 1, subBg: 'shadow', subColor: 'white', subPx: 24, subLift: 0, uiScale: 'm', dub: false, heroRotate: true, autoSkip: false, presence: true, tray: true, updates: true }, JSON.parse(localStorage.getItem('settings') || '{}')),
    save() { localStorage.setItem('settings', JSON.stringify(this.data)); },
    set(k, v) { this.data[k] = v; this.save(); },
    push() { return api.post('settings', { max_height: this.data.maxHeight, subtitle_lang: this.data.subtitleLang, audio_lang: this.data.audioLang || '' }).catch(() => {}); },
  };
  const S = Settings.data;

  const Memory = {
    map(k) { try { return JSON.parse(localStorage.getItem(k) || '{}'); } catch (_) { return {}; } },
    store(k, v) { localStorage.setItem(k, JSON.stringify(v)); },
    key: t => `${t.kind}-${t.id}`,
    preferredSource(t) { return this.map('sources')[this.key(t)]; },
    rememberSource(t, s) { const m = this.map('sources'); m[this.key(t)] = s; this.store('sources', m); },
    lastPlayed(t) { return this.map('last')[this.key(t)]; },
    remember(p) {
      const m = this.map('last');
      m[`${p.kind}-${p.id}`] = { title: p.title, season: p.season, episode: p.episode, position: p.position, duration: p.duration, at: Date.now() };
      this.store('last', m);
      if (p.duration > 0 && p.duration - p.position < 30) this.markWatched(`${p.season}-${p.episode}`, `${p.kind}-${p.id}`);
    },
    watched(key) { return new Set(this.map('watched')[key] || []); },
    markWatched(ep, key) { const m = this.map('watched'); const s = new Set(m[key] || []); s.add(ep); m[key] = Array.from(s); this.store('watched', m); },
    subtitle(t) { return this.map('subs')[this.key(t)]; },
    rememberSubtitle(t, lang) { const m = this.map('subs'); m[this.key(t)] = lang == null ? 'off' : lang; this.store('subs', m); },
    recent() { try { return JSON.parse(localStorage.getItem('recent') || '[]'); } catch (_) { return []; } },
    addRecent(q) { const r = this.recent().filter(x => x.toLowerCase() !== q.toLowerCase()); r.unshift(q); localStorage.setItem('recent', JSON.stringify(r.slice(0, 8))); },
  };

  const LANGS = { en: 'English', es: 'Spanish', fr: 'French', de: 'German', pt: 'Portuguese', it: 'Italian', ja: 'Japanese', ar: 'Arabic', ru: 'Russian', ko: 'Korean', zh: 'Chinese', tr: 'Turkish', pl: 'Polish', nl: 'Dutch', sv: 'Swedish', el: 'Greek', ro: 'Romanian', hi: 'Hindi', th: 'Thai', vi: 'Vietnamese', id: 'Indonesian' };
  const langName = code => { const c = String(code || '').toLowerCase(); return LANGS[c] || LANGS[c.slice(0, 2)] || (code || 'Unknown'); };
  const langMatch = (lang, pref) => { const a = String(lang || '').toLowerCase(), b = String(pref || '').toLowerCase(); return !!b && (a === b || a.slice(0, 2) === b.slice(0, 2)); };
  const SOURCE_NAMES = { vixsrc: 'Vix', cno: 'Cino', mapple: 'Mapple', vidbolt: 'Bolt', vidrock: 'Rock', animekai: 'Kai', animeparadise: 'Paradise', luffy: 'Luffy', kototv: 'Koto', offline: 'Downloaded' };
  const sourceLabel = s => SOURCE_NAMES[s] || (s ? s[0].toUpperCase() + s.slice(1) : '');
  const kindLabel = k => k === 'movie' ? 'Movie' : k === 'anime' ? 'Anime' : 'Series';
  const numText = n => String(n);
  const epTag = (ep, anime) => anime ? `E${numText(ep.number)}` : `S${ep.season} · E${Math.trunc(ep.number)}`;
  const epLabel = (ep, anime) => ep.name ? `${epTag(ep, anime)} · ${ep.name}` : epTag(ep, anime);
  const tkey = t => `${t.kind}-${t.id}`;

  const Item = {
    make(title, episodes, index, dub, startAt, chain, offline) { return { title, episodes: episodes || [], index: index || 0, dub: !!dub, startAt: startAt || null, chain: chain || [], offline: offline || null }; },
    ep: it => it.episodes[it.index] || null,
    isAnime: it => it.title.kind === 'anime',
    isMovie: it => it.title.kind === 'movie',
    heading(it) { const e = this.ep(it); return e ? `${it.title.name} · ${epLabel(e, this.isAnime(it))}${it.dub ? ' (dub)' : ''}` : it.title.name; },
    next(it) { if (this.isMovie(it) || !it.episodes[it.index + 1]) return null; return this.make(it.title, it.episodes, it.index + 1, it.dub, null, it.chain); },
    at(it, i) { return this.make(it.title, it.episodes, i, it.dub, null, it.chain); },
    request(it) { const e = this.ep(it); return { kind: it.title.kind, id: it.title.id, title: this.heading(it), season: e ? e.season : 0, episode: e ? e.number : 0, dub: it.dub, sources: e ? e.sources || [] : [] }; },
    progress(it, position, duration) { const e = this.ep(it); const name = this.isAnime(it) ? (e && e.name ? e.name : it.title.name) : it.title.name; return { kind: it.title.kind, id: it.title.id, season: e ? e.season : 0, episode: e ? Math.trunc(e.number) : 0, position, duration, title: name }; },
    downloadKey(it) { const e = this.ep(it); if (!e) return `${it.title.kind}-${it.title.id}`; return `${it.title.kind}-${it.title.id}-${e.season}-${numText(e.number).replace('.', '_')}${it.dub ? '-dub' : ''}`; },
    downloadMeta(it) { const e = this.ep(it); return { name: it.title.name, label: e ? epLabel(e, this.isAnime(it)) + (it.dub ? ' (dub)' : '') : (it.title.year || kindLabel(it.title.kind)), kind: it.title.kind, id: it.title.id, season: e ? e.season : 0, episode: e ? e.number : 0, poster: it.title.poster, still: e ? e.still : null, dub: it.dub }; },
  };

  const App = { boot: {}, user: null, mode: 'home', page: null, home: null, homeAt: 0, history: null, historyAt: 0, downloads: [], active: {}, watchlist: null, titles: {}, details: {}, lists: {} };
  const content = byId('content');
  const heroBox = byId('hero');

  const remember = t => { App.titles[tkey(t)] = t; return t; };
  const setUser = name => { App.user = name || null; document.body.classList.toggle('authed', !!App.user); };
  const metaHtml = t => {
    const parts = [];
    const g = (t.genres || [])[0];
    if (g) parts.push(`<span class="mi g">${esc(g)}</span>`);
    parts.push(`<span class="mi">${t.kind === 'movie' ? I.film : I.tv}${esc(t.year || kindLabel(t.kind))}</span>`);
    if (t.rating > 0) parts.push(`<span class="mi star">${I.star}${Number(t.rating).toFixed(1)}</span>`);
    return parts.join('');
  };
  const hoverHtml = t => `<span class="card-hover"><span class="pl">${I.play}</span><span class="ht">${esc(t.name)}</span>${(t.year || t.rating > 0) ? `<span class="hm">${[t.year, t.rating > 0 ? '★ ' + Number(t.rating).toFixed(1) : ''].filter(Boolean).join(' · ')}</span>` : ''}</span>`;
  const inList = t => !!(App.watchlist || []).find(w => w.kind === t.kind && w.id === t.id);
  const bookmarkHtml = t => `<button class="card-bookmark${inList(t) ? ' active' : ''}" type="button" aria-label="My List" data-key="${tkey(t)}">${I.bookmark}</button>`;

  function cardHtml(t) {
    remember(t);
    return `<a class="card" data-key="${tkey(t)}"><div class="card-poster">${t.poster ? `<img loading="lazy" src="${esc(t.poster)}" alt="">` : ''}${hoverHtml(t)}${bookmarkHtml(t)}</div><div class="card-title">${esc(t.name)}</div><div class="card-meta">${metaHtml(t)}</div></a>`;
  }
  function top10Html(t, rank) {
    remember(t);
    return `<a class="top10-card" data-key="${tkey(t)}"><div class="top10-poster"><div class="clip">${t.poster ? `<img loading="lazy" src="${esc(t.poster)}" alt="">` : ''}</div><span class="top10-rank"><small>Top</small><b>${String(rank).padStart(2, '0')}</b></span>${hoverHtml(t)}</div><div class="info"><div class="t">${esc(t.name)}</div><div class="m">${metaHtml(t)}</div></div></a>`;
  }
  function cwHtml(r) {
    const t = remember({ id: r.id, kind: r.kind, name: r.show, poster: r.poster, year: '', rating: 0, genres: [] });
    const pct = r.duration ? Math.min(100, r.position / r.duration * 100) : 0;
    const tag = r.kind === 'movie' ? `${clock(Math.max(r.duration - r.position, 0))} left` : `${r.kind === 'anime' ? `E${r.episode}` : `S${r.season} · E${r.episode}`} · ${clock(Math.max(r.duration - r.position, 0))} left`;
    return `<a class="card cw-card" data-key="${tkey(t)}" data-resume="1"><button class="cw-x" aria-label="Remove" title="Remove">✕</button><div class="card-poster">${t.poster ? `<img loading="lazy" src="${esc(t.poster)}" alt="">` : ''}<span class="ep-tag">${esc(tag)}</span><div class="cw-bar"><i style="width:${pct}%"></i></div></div><div class="card-title">${esc(t.name)}</div></a>`;
  }

  function railEl(title, html, opts) {
    opts = opts || {};
    const sec = el(`<section class="rail reveal in"${opts.id ? ` id="${opts.id}"` : ''}><div class="rail-head"><h2 class="rail-title">${esc(title)}</h2>${opts.side ? `<div class="rail-side">${opts.side}</div>` : ''}</div><div class="rail-wrap"><button class="rail-arrow left hidden" aria-label="Scroll left">${I.left}</button><button class="rail-arrow right" aria-label="Scroll right">${I.right}</button><div class="rail-scroll${opts.top10 ? ' is-top10' : ''}">${html}</div></div></section>`);
    const strip = $('.rail-scroll', sec), l = $('.rail-arrow.left', sec), r = $('.rail-arrow.right', sec);
    const sync = () => { l.classList.toggle('hidden', strip.scrollLeft < 10); r.classList.toggle('hidden', strip.scrollLeft + strip.clientWidth >= strip.scrollWidth - 10); };
    l.onclick = () => strip.scrollBy({ left: -strip.clientWidth * 0.85, behavior: 'smooth' });
    r.onclick = () => strip.scrollBy({ left: strip.clientWidth * 0.85, behavior: 'smooth' });
    strip.addEventListener('scroll', sync, { passive: true });
    setTimeout(sync, 50);
    return sec;
  }
  const skeletonRail = title => railEl(title, Array.from({ length: 8 }).map(() => '<div class="card"><div class="card-poster skel"></div><div class="card-title skel" style="height:14px;margin-top:9px;border-radius:3px"></div></div>').join(''));

  content.addEventListener('click', e => {
    const bm = e.target.closest('.card-bookmark');
    if (bm) { e.preventDefault(); e.stopPropagation(); toggleList(App.titles[bm.dataset.key], bm); return; }
    const x = e.target.closest('.cw-x');
    if (x) { e.preventDefault(); e.stopPropagation(); const card = x.closest('.cw-card'); const t = App.titles[card.dataset.key]; card.style.transition = 'opacity .2s,transform .2s'; card.style.opacity = '0'; card.style.transform = 'scale(.85)'; api.post('history/remove', { kind: t.kind, id: t.id }).catch(() => {}); App.homeAt = 0; setTimeout(() => card.remove(), 200); return; }
    const card = e.target.closest('.card, .top10-card, .lcard');
    if (!card) return;
    e.preventDefault();
    const t = App.titles[card.dataset.key];
    if (!t) return;
    if (e.target.closest('.card-hover .pl') || card.dataset.resume) openInfo(t, { autoplay: true });
    else openInfo(t);
  });

  async function toggleList(t, btn) {
    if (!t) return;
    if (!App.user) { openAccount(); return; }
    try {
      if (inList(t)) { await api.post('watchlist/remove', { kind: t.kind, id: t.id }); App.watchlist = (App.watchlist || []).filter(w => !(w.kind === t.kind && w.id === t.id)); toast('Removed from My List'); }
      else { await api.post('watchlist', { kind: t.kind, id: t.id, name: t.name, poster: t.poster }); App.watchlist = (App.watchlist || []).concat([{ id: t.id, kind: t.kind, name: t.name, poster: t.poster }]); toast('Added to My List'); }
      $$(`.card-bookmark[data-key="${tkey(t)}"]`).forEach(b => b.classList.toggle('active', inList(t)));
      $$('.ip-round.mylist').forEach(b => b.classList.toggle('active', inList(t)));
      if (btn) btn.classList.toggle('active', inList(t));
    } catch (err) { toast(String(err)); }
  }

  let heroItems = [], heroIdx = 0, heroTimer = null, heroGen = 0, pageGen = 0, ambientIdx = 0;
  function setAmbient(url) {
    const imgs = $$('#ambient img');
    if (!url) return;
    if (imgs[ambientIdx].getAttribute('src') === url && imgs[ambientIdx].classList.contains('on')) return;
    const nextIdx = imgs[ambientIdx].getAttribute('src') === url ? ambientIdx : 1 - ambientIdx;
    const next = imgs[nextIdx], prev = imgs[1 - nextIdx];
    const show = () => { ambientIdx = nextIdx; next.classList.add('on'); prev.classList.remove('on'); };
    if (next.getAttribute('src') !== url) next.src = url;
    if (next.complete && next.naturalWidth) show(); else { next.onload = show; next.onerror = () => {}; }
  }
  function heroMeta(t) {
    const runtime = t.runtime ? (t.kind === 'tv' ? `${t.runtime}m` : human(t.runtime)) : '';
    return [
      t.rating > 0 ? `<span class="m">${Math.round(Number(t.rating) * 10)}% match</span>` : '',
      t.year ? `<span>${esc(t.year)}</span>` : '',
      `<span class="t">${kindLabel(t.kind)}</span>`,
      t.season_count > 1 ? `<span>${t.season_count} seasons</span>` : (runtime ? `<span>${runtime}</span>` : ''),
      (t.genres || []).length ? `<span>${t.genres.slice(0, 2).map(esc).join(' · ')}</span>` : '',
    ].filter(Boolean).join('');
  }
  function renderHero() {
    const gen = ++heroGen;
    heroBox.classList.remove('collapsed');
    if (!heroItems.length) { heroBox.innerHTML = ''; return; }
    heroBox.innerHTML = heroItems.map((t, i) => {
      const over = (t.overview || '').length > 200 ? t.overview.slice(0, 197).trim() + '…' : (t.overview || '');
      const title = t.logo ? `<img class="hero-logo" src="${esc(t.logo)}" alt="">` : `<h1 class="${t.name.length > 28 ? 'sm' : t.name.length > 16 ? 'md' : ''}">${esc(t.name)}</h1>`;
      return `<div class="hero-slide${i === heroIdx ? ' active' : ''}" data-i="${i}"><div class="hero-img" style="--bd:url('${esc(t.backdrop || t.poster || '')}')"></div><div class="hero-vig"></div><div class="hero-in">${title}<div class="hero-meta">${heroMeta(t)}</div>${over ? `<p class="hero-overview">${esc(over)}</p>` : ''}<div class="hero-cta"><button class="btn play" data-play="${i}">${I.play}Play</button><button class="btn info" data-info="${i}">${I.info}More Info</button></div></div></div>`;
    }).join('') + `<div class="hero-dots">${heroItems.map((_, i) => `<span class="hero-dot${i === heroIdx ? ' active' : ''}" data-i="${i}"></span>`).join('')}</div>`;
    setAmbient(heroItems[heroIdx].backdrop || heroItems[heroIdx].poster);
    document.documentElement.classList.toggle('hero-still', !S.heroRotate);
    $$('.hero-dot', heroBox).forEach(d => d.onclick = () => { heroIdx = Number(d.dataset.i); renderHero(); });
    $$('[data-play]', heroBox).forEach(b => b.onclick = () => openInfo(heroItems[Number(b.dataset.play)], { autoplay: true }));
    $$('[data-info]', heroBox).forEach(b => b.onclick = () => openInfo(heroItems[Number(b.dataset.info)]));
    clearTimeout(heroTimer);
    if (S.heroRotate && heroItems.length > 1) heroTimer = setTimeout(() => { if (gen === heroGen && !document.hidden) { heroIdx = (heroIdx + 1) % heroItems.length; renderHero(); } }, 9000);
    const pending = heroItems.filter(t => t.logo === undefined && t.kind);
    if (pending.length) Promise.all(pending.map(t => detailsFor(t).then(d => { if (d) Object.assign(t, { logo: d.logo || null, backdrop: t.backdrop || d.backdrop, overview: (t.overview && !/^\d+ episodes$/.test(t.overview)) ? t.overview : (d.overview || t.overview), runtime: d.runtime, season_count: d.season_count, genres: d.genres }); else t.logo = null; }))).then(() => { if (gen === heroGen) renderHero(); });
  }
  async function detailsFor(t) {
    const k = tkey(t);
    if (App.details[k]) return App.details[k];
    try { const d = await api.get('details', { kind: t.kind, id: String(t.id) }); App.details[k] = d; return d; } catch (_) { return null; }
  }

  async function listRail(title, path) {
    let rows = App.lists[path];
    if (!rows) { try { rows = await api.get('list', { path }); App.lists[path] = rows; } catch (_) { rows = []; } }
    if (!rows.length) return null;
    return railEl(title, rows.slice(0, 20).map(cardHtml).join(''));
  }

  async function showMode(mode) {
    App.mode = mode; App.page = null;
    $$('.tab[data-mode]').forEach(b => b.classList.toggle('on', b.dataset.mode === mode));
    $$('.nicon').forEach(b => b.classList.remove('on'));
    window.scrollTo({ top: 0 });
    content.innerHTML = '';
    heroBox.classList.remove('collapsed');
    heroBox.innerHTML = '<div class="hero-slide active"><div class="hero-img skel"></div></div>';
    const gen = ++pageGen;
    const add = r => { if (r && gen === pageGen) content.appendChild(r); };
    if (mode === 'home') {
      if (!App.home || Date.now() - App.homeAt > 300000) {
        [1, 2, 3].forEach(() => content.appendChild(skeletonRail(' ')));
        try { App.home = await api.get('home'); App.homeAt = Date.now(); } catch (e) { content.innerHTML = `<div class="rail-empty big"><b>Can't reach viel</b>${esc(e)}</div>`; return; }
        if (gen !== pageGen) return;
        content.innerHTML = '';
      }
      const d = App.home;
      heroItems = [];
      if (d.hero) heroItems.push(Object.assign({}, d.hero));
      (d.top || []).forEach(t => { if (!heroItems.find(h => h.id === t.id && h.kind === t.kind) && heroItems.length < 6) heroItems.push(Object.assign({}, t)); });
      heroIdx = 0; renderHero();
      const cont = (d.continue || []).filter(r => r.duration > 0);
      if (cont.length) add(railEl('Continue Watching', cont.map(cwHtml).join(''), { id: 'rail-continue' }));
      if (App.watchlist === null) { try { App.watchlist = await api.get('watchlist'); } catch (_) { App.watchlist = []; } if (gen !== pageGen) return; }
      if (App.watchlist.length) add(railEl('My List', App.watchlist.slice(0, 20).map(t => cardHtml(Object.assign({ year: '', rating: 0, genres: [] }, t))).join('')));
      add(railEl('Top 10 Today', (d.top || []).slice(0, 10).map((t, i) => top10Html(t, i + 1)).join(''), { top10: true }));
      add(railEl('Trending This Week', (d.trending || []).map(cardHtml).join('')));
      add(railEl('Popular Series', (d.popular_tv || []).map(cardHtml).join('')));
      add(railEl('Anime', (d.anime || []).map(cardHtml).join('')));
      add(railEl('Top Rated Movies', (d.top_rated || []).map(cardHtml).join('')));
      api.get('recommendations').then(rows => { if (gen === pageGen && rows && rows.length) add(railEl('Because You Watched', rows.map(cardHtml).join(''))); }).catch(() => {});
      return;
    }
    const plan = mode === 'movie' ? [
      ['Popular Movies', 'movie/popular'], ['Now Playing', 'movie/now_playing'], ['Top Rated', 'movie/top_rated'], ['Upcoming', 'movie/upcoming'],
      ['Action', 'discover/movie?with_genres=28&sort_by=popularity.desc'], ['Comedy', 'discover/movie?with_genres=35&sort_by=popularity.desc'], ['Horror', 'discover/movie?with_genres=27&sort_by=popularity.desc'], ['Sci-Fi', 'discover/movie?with_genres=878&sort_by=popularity.desc'], ['Animation', 'discover/movie?with_genres=16&sort_by=popularity.desc'],
    ] : mode === 'tv' ? [
      ['Popular Series', 'tv/popular'], ['Airing Now', 'tv/on_the_air'], ['Top Rated', 'tv/top_rated'],
      ['Action & Adventure', 'discover/tv?with_genres=10759&sort_by=popularity.desc'], ['Drama', 'discover/tv?with_genres=18&sort_by=popularity.desc'], ['Comedy', 'discover/tv?with_genres=35&sort_by=popularity.desc'], ['Crime', 'discover/tv?with_genres=80&sort_by=popularity.desc'], ['Sci-Fi & Fantasy', 'discover/tv?with_genres=10765&sort_by=popularity.desc'],
    ] : [['Trending Anime', 'anime'], ['More Anime', 'anime?page=2'], ['Keep Going', 'anime?page=3']];
    const first = plan[0];
    let rows = App.lists[first[1]];
    if (!rows) { try { rows = await api.get('list', { path: first[1] }); App.lists[first[1]] = rows; } catch (_) { rows = []; } }
    if (gen !== pageGen) return;
    heroItems = rows.slice(0, 6).map(t => Object.assign({}, t)); heroIdx = 0; renderHero();
    if (mode === 'anime') {
      const cont = ((App.home || {}).continue || []).filter(r => r.kind === 'anime' && r.duration > 0);
      if (cont.length) add(railEl('Continue Watching', cont.map(cwHtml).join('')));
    }
    for (const [title, path] of plan) {
      const r = await listRail(title, path);
      if (gen !== pageGen) return;
      add(r);
    }
  }

  async function showPage(name) {
    App.page = name;
    $$('.tab[data-mode]').forEach(b => b.classList.remove('on'));
    $$('.nicon').forEach(b => b.classList.toggle('on', b.id === `nav-${name}`));
    ++pageGen; ++heroGen; clearTimeout(heroTimer);
    heroBox.classList.add('collapsed'); heroBox.innerHTML = '';
    window.scrollTo({ top: 0 });
    content.innerHTML = '';
    if (name === 'downloads') return renderDownloads();
    if (name === 'history') return renderHistory();
  }

  async function renderDownloads() {
    let rows = [];
    try { rows = await api.get('downloads'); } catch (_) {}
    App.downloads = rows;
    if (App.page !== 'downloads') return;
    const head = el(`<div class="page-head"><div><h1>Downloads</h1><div class="sub">Saved on this PC. They play without a connection.</div></div><button class="act" id="dl-folder">${I.folder}Open folder</button></div>`);
    $('#dl-folder', head).onclick = () => invoke('open_downloads');
    content.innerHTML = '';
    content.appendChild(head);
    if (!rows.length) { content.appendChild(el('<div class="rail-empty big"><b>Nothing downloaded yet</b>Use the download button on any episode or movie.</div>')); return; }
    const list = el('<div class="lrows"></div>');
    rows.forEach(r => list.appendChild(downloadRow(r)));
    content.appendChild(list);
  }
  function downloadRow(r) {
    const m = r.meta || {};
    const live = App.active[r.key];
    const state = live ? live.state : r.state;
    const done = live ? live.done : r.done, total = live ? live.total : r.total;
    const frac = total ? done / total : 0;
    const failed = state === 'failed' || String(state).startsWith('failed');
    const status = state === 'done' ? `${size(r.bytes)} · ${sourceLabel(r.source)}` : failed ? `Failed · ${r.error || String(state).replace('failed: ', '')}` : state === 'resolving' ? 'Finding a source…' : total ? `${Math.round(frac * 100)}% · ${done}/${total} parts` : 'Starting…';
    const row = el(`<div class="lrow" data-key="${esc(r.key)}"><div class="po">${m.poster ? `<img src="${esc(m.poster)}" alt="">` : ''}</div><div><div class="n">${esc(m.name || r.key)}</div><div class="s">${esc(m.label || '')} · <span class="st">${esc(status)}</span></div>${state !== 'done' && !failed ? `<div class="bar"><i style="width:${Math.round(frac * 100)}%"></i></div>` : ''}</div><div class="acts">${state === 'done' ? `<button title="Play">${I.play}</button>` : ''}<button title="Remove">${state === 'done' || failed ? I.trash : I.x}</button></div></div>`);
    row.addEventListener('click', async e => {
      if (e.target.closest('[title=Remove]')) { if (App.active[r.key]) invoke('download_cancel', { key: r.key }); else await api.post('downloads/remove', { key: r.key }).catch(() => {}); row.remove(); return; }
      if (state === 'done') playOffline(r);
    });
    return row;
  }
  function playOffline(r) {
    const m = r.meta || {};
    const title = { id: m.id, kind: m.kind, name: m.name, poster: m.poster, year: '' };
    const episodes = m.kind === 'movie' ? [] : [{ season: m.season || 0, number: m.episode || 0, name: m.label || '', overview: '', still: m.still, sources: [] }];
    Player.start(Item.make(title, episodes, 0, !!m.dub, null, [], r.key));
  }

  async function loadHistory(force) {
    if (!force && App.history && Date.now() - App.historyAt < 45000) return App.history;
    try { App.history = await api.get('history'); App.historyAt = Date.now(); } catch (_) { App.history = App.history || []; }
    return App.history;
  }
  async function renderHistory() {
    content.innerHTML = '<div class="page-head"><div><h1>History</h1><div class="sub">Everything you have watched, synced with the site.</div></div></div>';
    const rows = await loadHistory(true);
    if (App.page !== 'history') return;
    if (!rows.length) { content.appendChild(el('<div class="rail-empty big"><b>No history yet</b>Start something and it shows up here.</div>')); return; }
    const list = el('<div class="lrows"></div>');
    rows.forEach(r => {
      const t = remember({ id: r.id, kind: r.kind, name: r.show, poster: r.poster, year: '', rating: 0, genres: [] });
      const tag = r.kind === 'movie' ? '' : r.kind === 'anime' ? `E${r.episode}` : `S${r.season} · E${r.episode}`;
      const done = r.duration > 0 && r.duration - r.position < 30;
      const row = el(`<div class="lrow"><div class="po">${r.poster ? `<img src="${esc(r.poster)}" alt="">` : ''}</div><div><div class="n">${esc(r.show)}</div><div class="s">${esc([tag, done ? 'Finished' : r.duration ? `${clock(Math.max(r.duration - r.position, 0))} left` : '', new Date(r.updated_at * 1000).toLocaleDateString()].filter(Boolean).join(' · '))}</div>${r.duration && !done ? `<div class="bar"><i style="width:${Math.round(r.position / r.duration * 100)}%"></i></div>` : ''}</div><div class="acts"><button title="Play">${I.play}</button><button title="Remove">${I.trash}</button></div></div>`);
      row.addEventListener('click', async e => {
        if (e.target.closest('[title=Remove]')) { await api.post('history/remove', { kind: r.kind, id: r.id }).catch(() => {}); row.remove(); App.homeAt = 0; return; }
        openInfo(t, { autoplay: !!e.target.closest('[title=Play]'), row: r });
      });
      list.appendChild(row);
    });
    content.appendChild(list);
  }

  const ov = byId('ov'), searchInput = byId('nav-search-input'), searchGrid = byId('search-grid'), searchTop = byId('search-top');
  let searchSeq = 0, searchTimer;
  function openSearch() { ov.classList.add('open'); renderRecent(); setTimeout(() => { searchInput.focus(); searchInput.select(); }, 30); }
  function closeSearch() { ov.classList.remove('open'); }
  function renderRecent() {
    const r = Memory.recent();
    byId('search-recent-label').hidden = !r.length;
    byId('search-recent').innerHTML = r.map(q => `<button class="schip">${esc(q)}</button>`).join('');
    $$('#search-recent .schip').forEach(b => b.onclick = () => { searchInput.value = b.textContent; runSearch(); });
  }
  async function runSearch() {
    const q = searchInput.value.trim();
    const my = ++searchSeq;
    $('.sp', ov).classList.toggle('has-q', !!q);
    if (!q) { searchGrid.innerHTML = ''; searchTop.innerHTML = ''; byId('search-results-title').hidden = true; return; }
    let rows = [];
    try { rows = await api.get('search', { q }); } catch (_) {}
    if (my !== searchSeq) return;
    Memory.addRecent(q);
    byId('search-results-title').hidden = false;
    byId('search-results-title').textContent = rows.length ? `Results for “${q}”` : `Nothing for “${q}”`;
    rows.forEach(remember);
    const top = rows[0];
    searchTop.innerHTML = top ? `<div class="po">${top.poster ? `<img src="${esc(top.poster)}" alt="">` : ''}</div><div class="tx"><div class="n">${esc(top.name)}</div><div class="m">${[kindLabel(top.kind), top.year, top.rating > 0 ? `★ ${Number(top.rating).toFixed(1)}` : ''].filter(Boolean).map(x => `<span class="c">${esc(x)}</span>`).join('')}</div>${top.overview ? `<p class="o">${esc(top.overview)}</p>` : ''}<div class="a"><button class="pl" data-key="${tkey(top)}">${I.play}Play</button><button class="rd" data-info="${tkey(top)}">${I.info}</button><button class="rd${inList(top) ? ' on' : ''}" data-list="${tkey(top)}">${I.plus}</button></div></div>` : '';
    searchGrid.innerHTML = rows.slice(1).map(t => `<a class="scard" data-key="${tkey(t)}"><div class="scard-art">${t.poster ? `<img loading="lazy" src="${esc(t.poster)}" alt="">` : ''}<span class="scard-hover"><span class="n">${esc(t.name)}${t.year ? ` (${esc(t.year)})` : ''}</span>${(t.genres || []).length ? `<span class="c">${esc(t.genres.slice(0, 3).join(', '))}</span>` : ''}<span class="tags">${[kindLabel(t.kind), t.year, t.rating > 0 ? `★ ${Number(t.rating).toFixed(1)}` : ''].filter(Boolean).map(x => `<span>${esc(x)}</span>`).join('')}</span></span></div><div class="scard-meta"><div class="n">${esc(t.name)}</div><div class="s">${esc([kindLabel(t.kind), t.year].filter(Boolean).join(' · '))}</div></div></a>`).join('');
  }
  searchInput.addEventListener('input', () => { clearTimeout(searchTimer); searchTimer = setTimeout(runSearch, 260); });
  searchInput.addEventListener('keydown', e => { if (e.key === 'Enter') { clearTimeout(searchTimer); runSearch(); } });
  ov.addEventListener('click', e => {
    if (e.target.classList.contains('ov-bg')) { closeSearch(); return; }
    const pl = e.target.closest('[data-key].pl'); if (pl) { closeSearch(); openInfo(App.titles[pl.dataset.key], { autoplay: true }); return; }
    const inf = e.target.closest('[data-info]'); if (inf) { closeSearch(); openInfo(App.titles[inf.dataset.info]); return; }
    const ls = e.target.closest('[data-list]'); if (ls) { toggleList(App.titles[ls.dataset.list]).then(() => ls.classList.toggle('on', inList(App.titles[ls.dataset.list]))); return; }
    const card = e.target.closest('.scard'); if (card) { e.preventDefault(); closeSearch(); openInfo(App.titles[card.dataset.key]); }
  });

  const infoOv = byId('info-ov'), infoPanel = byId('info-panel');
  let info = null;
  function closeInfo() { infoOv.classList.add('closing'); setTimeout(() => { infoOv.classList.remove('open', 'closing'); infoPanel.innerHTML = ''; info = null; }, 200); }
  infoOv.addEventListener('click', e => { if (e.target.classList.contains('ov-bg') || e.target.closest('[data-close]')) closeInfo(); });

  async function openInfo(t, opts) {
    opts = opts || {};
    if (!t) return;
    const key = tkey(t);
    infoOv.classList.add('open');
    infoPanel.scrollTop = 0;
    infoPanel.innerHTML = `<div class="ip-hero"><div class="ip-backdrop" style="background-image:url('${esc(t.backdrop || t.poster || '')}')"></div><div class="ip-vig"></div><button class="ip-close" data-close aria-label="Close">${I.x}</button><div class="ip-hero-in"><h2>${esc(t.name)}</h2></div></div><div class="ip-body"><div class="ip-kv ip-skel">&nbsp;</div><div class="ip-kv ip-skel">&nbsp;</div></div>`;
    info = { key, t };
    let d, list = { episodes: [], dub: false };
    try {
      const jobs = [detailsFor(t)];
      if (t.kind !== 'movie') jobs.push(api.get('title', { kind: t.kind, id: String(t.id), dub: S.dub && t.kind === 'anime' ? '1' : '0' }).catch(() => list));
      jobs.push(loadHistory(false));
      if (App.watchlist === null) jobs.push(api.get('watchlist').then(r => { App.watchlist = r; }).catch(() => { App.watchlist = []; }));
      const out = await Promise.all(jobs);
      d = out[0];
      if (t.kind !== 'movie') list = out[1] || list;
    } catch (e) { d = null; }
    if (!info || info.key !== key) return;
    if (!d) { infoPanel.innerHTML = `<div class="ip-body" style="padding-top:60px"><button class="ip-close" data-close aria-label="Close">${I.x}</button><div class="rail-empty big"><b>Couldn't load this title</b>Check the connection and try again.</div></div>`; return; }
    const title = remember({ id: d.id, kind: d.kind, name: d.name, year: d.year, rating: d.rating, overview: d.overview, poster: d.poster, backdrop: d.backdrop, genres: d.genres, seasons: d.seasons });
    Object.assign(info, { d, title, list, row: opts.row || rowFor(t.kind, t.id), season: null, watched: Memory.watched(key), downloaded: new Set((App.downloads || []).filter(x => x.state === 'done').map(x => x.key)) });
    api.get('downloads').then(rows => { App.downloads = rows; if (info && info.key === key) { info.downloaded = new Set(rows.filter(x => x.state === 'done').map(x => x.key)); renderEpisodes(); } }).catch(() => {});
    renderInfo();
    if (opts.autoplay) { const item = resumeItem(info); if (item) Player.start(item); }
  }

  function rowFor(kind, id) {
    const hist = (App.history || []).find(r => r.kind === kind && r.id === id);
    if (hist) return hist;
    const local = Memory.lastPlayed({ kind, id });
    if (local) return { id, kind, show: local.title, season: local.season, episode: local.episode, position: local.position, duration: local.duration };
    return null;
  }
  function episodeIndex(st, row) {
    if (!row) return -1;
    return st.list.episodes.findIndex(e => st.title.kind === 'anime' ? Math.trunc(e.number) === row.episode : e.season === row.season && Math.trunc(e.number) === row.episode);
  }
  function resumeItem(st) {
    const { title, list, row, d } = st;
    if (title.kind === 'movie') return Item.make(title, [], 0, false, row && row.duration > 0 && row.duration - row.position > 30 ? row.position : null, d.seasons || []);
    const eps = list.episodes;
    if (!eps.length) return null;
    let idx = episodeIndex(st, row), start = null;
    if (idx >= 0) {
      if (row.duration > 0 && row.duration - row.position < 30) idx = Math.min(idx + 1, eps.length - 1);
      else if (row.position > 0) start = row.position;
    } else {
      const firstUnseen = eps.findIndex(e => !st.watched.has(`${e.season}-${Math.trunc(e.number)}`));
      idx = firstUnseen > 0 ? firstUnseen : 0;
    }
    return Item.make(title, eps, idx, list.dub, start, d.seasons || []);
  }
  function playLabel(st) {
    const item = resumeItem(st);
    if (!item) return { text: 'Play', item: null };
    if (item.title.kind === 'movie') return { text: item.startAt ? `Resume <small>${clock(item.startAt)}</small>` : 'Play', item };
    const e = Item.ep(item);
    return { text: `${item.startAt ? 'Resume' : 'Play'} <small>${e ? epTag(e, item.title.kind === 'anime') : ''}</small>`, item };
  }

  function renderInfo() {
    const st = info;
    const { d, title, list } = st;
    const play = playLabel(st);
    const chips = [
      d.year ? `<span class="y">${esc(d.year)}</span>` : '',
      '<span class="c">HD</span>',
      title.kind === 'tv' ? `<span class="c">${d.season_count ? `${d.season_count} season${d.season_count > 1 ? 's' : ''}` : 'Series'}</span>` : title.kind === 'anime' ? `<span class="c">${list.episodes.length ? `${list.episodes.length} eps` : 'Anime'}</span>` : (d.runtime ? `<span class="c">${human(d.runtime)}</span>` : '<span class="c">Movie</span>'),
      d.rating > 0 ? `<span class="c">★ ${Number(d.rating).toFixed(1)}</span>` : '',
    ].filter(Boolean).join('');
    infoPanel.innerHTML = `<div class="ip-hero"><div class="ip-backdrop" style="background-image:url('${esc(d.backdrop || d.poster || '')}')"></div><div class="ip-vig"></div><button class="ip-close" data-close aria-label="Close">${I.x}</button><div class="ip-hero-in">${d.logo ? `<img class="ip-logo" src="${esc(d.logo)}" alt="${esc(d.name)}">` : `<h2>${esc(d.name)}</h2>`}</div></div>
      <div class="ip-body"><div class="ip-chips">${chips}</div>
      <div class="ip-actions">
        <button class="ip-play${play.item && play.item.startAt ? ' resume' : ''}" id="ip-play">${I.play}${play.text}</button>
        <button class="ip-round mylist${inList(title) ? ' active' : ''}" id="ip-list" title="My List">${I.plus}${I.tick}</button>
        ${title.kind === 'movie' ? `<button class="ip-round${st.downloaded.has(`movie-${title.id}`) ? ' on' : ''}" id="ip-dl" title="Download">${I.download}</button>` : `<button class="ip-round" id="ip-more" title="More">${I.more}</button>`}
        <button class="ip-round" id="ip-share" title="Copy link">${I.share}</button>
      </div>
      ${d.tagline ? `<p class="ip-note">${esc(d.tagline)}</p>` : ''}
      ${d.overview ? `<p class="ip-overview">${esc(d.overview)}</p>` : ''}
      ${d.cast && d.cast.length ? `<div class="ip-kv"><b>Starring:</b> ${d.cast.slice(0, 6).map(esc).join(', ')}</div>` : ''}
      ${d.genres && d.genres.length ? `<div class="ip-kv"><b>Genres:</b> ${d.genres.map(esc).join(', ')}</div>` : ''}
      ${st.row && st.row.duration ? `<div class="ip-kv"><b>Last watched:</b> ${esc(st.row.kind === 'movie' ? clock(st.row.position) : epTag({ season: st.row.season, number: st.row.episode }, st.row.kind === 'anime') + ' · ' + clock(st.row.position))}</div>` : ''}
      ${title.kind !== 'movie' ? '<div class="ip-seasons"><div class="ip-season-pills" id="ip-pills"></div><div class="ip-eps" id="ip-eps"></div></div>' : ''}
      </div>`;
    byId('ip-play').onclick = () => { const p = playLabel(st); if (p.item) Player.start(p.item); else toast('No episodes listed yet'); };
    byId('ip-list').onclick = e => { const b = e.currentTarget; toggleList(title, null).then(() => b.classList.toggle('active', inList(title))); };
    byId('ip-share').onclick = () => navigator.clipboard.writeText(`${App.boot.host || 'https://viel.tv'}/${title.kind === 'movie' ? 'movies' : title.kind}/${title.id}`).then(() => toast('Link copied'));
    const dl = byId('ip-dl');
    if (dl) dl.onclick = () => startDownload(Item.make(title, [], 0, false, null, []), dl);
    const more = byId('ip-more');
    if (more) more.onclick = e => menuAt(e.currentTarget, [
      { label: 'Download this season', icon: I.download, run: () => downloadSeason(st, false) },
      { label: 'Download unwatched', icon: I.download, run: () => downloadSeason(st, true) },
      { label: 'Mark season as watched', icon: I.check, run: () => { visibleEpisodes(st).forEach(e => Memory.markWatched(`${e.season}-${Math.trunc(e.number)}`, st.key)); st.watched = Memory.watched(st.key); renderEpisodes(); } },
      title.kind === 'anime' ? { label: list.dub ? 'Switch to subbed' : 'Switch to dubbed', run: () => switchDub(st) } : null,
    ].filter(Boolean));
    renderEpisodes();
  }
  function visibleEpisodes(st) {
    const eps = st.list.episodes;
    if (st.title.kind === 'anime') return eps;
    const seasons = Array.from(new Set(eps.map(e => e.season)));
    if (st.season == null) { const idx = episodeIndex(st, st.row); st.season = idx >= 0 ? eps[idx].season : (seasons.find(s => s > 0) ?? seasons[0]); }
    return eps.filter(e => e.season === st.season);
  }
  function renderEpisodes() {
    const st = info;
    if (!st || !st.list || st.title.kind === 'movie') return;
    const pills = byId('ip-pills'), box = byId('ip-eps');
    if (!pills || !box) return;
    const { title, list, d } = st;
    const eps = list.episodes;
    let html = '';
    if (title.kind === 'tv') {
      visibleEpisodes(st);
      html = Array.from(new Set(eps.map(e => e.season))).map(s => `<button type="button" class="ip-pill${s === st.season ? ' on' : ''}" data-s="${s}">${s === 0 ? 'Specials' : `Season ${s}`}</button>`).join('');
    } else if (d.seasons && d.seasons.length > 1) {
      html = d.seasons.map(s => `<button type="button" class="ip-pill${s.id === title.id ? ' on' : ''}" data-a="${s.id}">${esc(s.title || `Season ${s.season}`)}</button>`).join('');
    }
    if (title.kind === 'anime') html += `<button type="button" class="ip-pill dub${list.dub ? ' on' : ''}" data-dub="1">${list.dub ? 'Dub' : 'Sub'}</button>`;
    pills.innerHTML = html;
    $$('.ip-pill', pills).forEach(b => b.onclick = () => {
      if (b.dataset.s != null) { st.season = Number(b.dataset.s); renderEpisodes(); }
      else if (b.dataset.a) { if (Number(b.dataset.a) !== title.id) openInfo({ kind: 'anime', id: Number(b.dataset.a), name: b.textContent }); }
      else if (b.dataset.dub) switchDub(st);
    });
    const visible = visibleEpisodes(st);
    if (!visible.length) { box.innerHTML = '<div class="ip-eps-msg">No episodes listed.</div>'; return; }
    const cur = episodeIndex(st, st.row);
    box.innerHTML = '';
    visible.forEach(e => {
      const i = eps.indexOf(e);
      const item = Item.make(title, eps, i, list.dub, null, d.seasons || []);
      const seen = st.watched.has(`${e.season}-${Math.trunc(e.number)}`);
      const isCur = i === cur && st.row && !(st.row.duration > 0 && st.row.duration - st.row.position < 30);
      const frac = isCur && st.row.duration ? st.row.position / st.row.duration : (seen ? 1 : 0);
      const have = st.downloaded.has(Item.downloadKey(item));
      const row = el(`<div class="ip-ep${isCur ? ' on' : ''}"><div class="th">${(e.still || d.backdrop) ? `<img loading="lazy" src="${esc(e.still || d.backdrop)}" alt="">` : ''}<span class="n">${esc(title.kind === 'anime' ? `E${numText(e.number)}` : `E${Math.trunc(e.number)}`)}</span>${frac > 0 ? `<div class="prog"><i style="width:${Math.round(frac * 100)}%"></i></div>` : ''}${seen && !isCur ? `<div class="seen">${I.check}</div>` : ''}</div><div class="tx"><div class="t">${esc(e.name || `Episode ${numText(e.number)}`)}</div>${e.overview ? `<div class="o">${esc(e.overview)}</div>` : ''}<div class="r">${esc([e.runtime || d.runtime ? human(e.runtime || d.runtime) : '', isCur ? `${clock(Math.max(st.row.duration - st.row.position, 0))} left` : ''].filter(Boolean).join(' · '))}</div></div><button class="dl${have ? ' done' : ''}" title="${have ? 'Downloaded' : 'Download'}">${have ? I.check : I.download}</button></div>`);
      row.addEventListener('click', ev => {
        if (ev.target.closest('.dl')) { if (!have) startDownload(item, ev.target.closest('.dl')); return; }
        Player.start(Item.make(title, eps, i, list.dub, isCur ? st.row.position : null, d.seasons || []));
      });
      box.appendChild(row);
    });
  }
  async function switchDub(st) {
    try {
      const list = await api.get('title', { kind: 'anime', id: String(st.title.id), dub: st.list.dub ? '0' : '1' });
      if (list.dub === st.list.dub) { toast(list.dub ? 'Only the dub is available' : 'No dub for this one'); return; }
      st.list = list; Settings.set('dub', list.dub); renderEpisodes();
    } catch (e) { toast(String(e)); }
  }

  async function startDownload(item, button) {
    if (!App.user) { openAccount(); return; }
    const key = Item.downloadKey(item);
    if (App.active[key]) { toast('Already downloading'); return; }
    const body = Object.assign(Item.request(item), { key, meta: Item.downloadMeta(item), max_height: S.downloadHeight });
    try {
      await invoke('download_start', { body });
      App.active[key] = { key, done: 0, total: 0, state: 'resolving' };
      updateDlDot();
      toast(`Downloading ${Item.heading(item)}`);
      if (button) { button.innerHTML = I.check; button.classList.add('done', 'on'); }
    } catch (e) { toast(String(e)); }
  }
  async function downloadSeason(st, onlyUnwatched) {
    const eps = visibleEpisodes(st).filter(e => !onlyUnwatched || !st.watched.has(`${e.season}-${Math.trunc(e.number)}`));
    let n = 0;
    for (const e of eps) {
      const item = Item.make(st.title, st.list.episodes, st.list.episodes.indexOf(e), st.list.dub, null, []);
      const key = Item.downloadKey(item);
      if (st.downloaded.has(key) || App.active[key]) continue;
      try { await invoke('download_start', { body: Object.assign(Item.request(item), { key, meta: Item.downloadMeta(item), max_height: S.downloadHeight }) }); App.active[key] = { key, done: 0, total: 0, state: 'resolving' }; n++; } catch (_) {}
      await sleep(400);
    }
    updateDlDot();
    toast(n ? `Queued ${n} episode${n === 1 ? '' : 's'}` : 'Nothing new to download');
  }
  function updateDlDot() { byId('dl-dot').hidden = !Object.keys(App.active).length; }
  listen('download', p => {
    const finished = p.state === 'done' || p.state === 'cancelled' || String(p.state).startsWith('failed');
    if (finished) {
      delete App.active[p.key];
      if (p.state === 'done') toast('Download finished');
      if (String(p.state).startsWith('failed')) toast(`Download failed: ${p.state.slice(8)}`);
      if (info && p.state === 'done') { info.downloaded.add(p.key); renderEpisodes(); }
      if (App.page === 'downloads') renderDownloads();
    } else {
      const fresh = !App.active[p.key];
      App.active[p.key] = p;
      const row = $(`.lrow[data-key="${p.key}"]`);
      if (row) { const bar = $('.bar i', row); if (bar && p.total) bar.style.width = `${Math.round(p.done / p.total * 100)}%`; const s = $('.st', row); if (s) s.textContent = p.total ? `${Math.round(p.done / p.total * 100)}% · ${p.done}/${p.total} parts` : 'Finding a source…'; }
      else if (App.page === 'downloads' && fresh) renderDownloads();
    }
    updateDlDot();
  });

  const ctx = byId('ctx');
  const zoomOf = () => Number(getComputedStyle(document.documentElement).getPropertyValue('--z')) || 1;
  function menuAt(anchor, items, opts) {
    opts = opts || {};
    closeMenus();
    const m = opts.into || ctx;
    m.innerHTML = (opts.title ? `<h4>${esc(opts.title)}</h4>` : '') + items.map((it, i) => it === 'hr' ? '<hr>' : `<button class="mi${it.on ? ' on' : ''}" data-i="${i}">${it.icon || ''}<span>${esc(it.label)}</span>${it.toggle ? '<i class="tg"></i>' : ''}${it.hint ? `<small>${esc(it.hint)}</small>` : ''}</button>`).join('');
    m.hidden = false;
    m.onclick = e => { const b = e.target.closest('.mi'); if (!b) return; const it = items[Number(b.dataset.i)]; closeMenus(); if (it.run) it.run(); };
    const r = anchor.getBoundingClientRect();
    if (m === ctx) {
      const z = zoomOf();
      const vw = window.innerWidth / z, vh = window.innerHeight / z;
      m.style.left = `${Math.min(r.left / z, vw - m.offsetWidth - 12)}px`;
      m.style.top = `${r.bottom / z + 6 + m.offsetHeight > vh ? r.top / z - m.offsetHeight - 6 : r.bottom / z + 6}px`;
    } else {
      const host = m.offsetParent.getBoundingClientRect();
      m.style.left = `${clamp(r.left - host.left + r.width / 2 - m.offsetWidth / 2, 12, host.width - m.offsetWidth - 12)}px`;
      m.style.top = 'auto';
      m.style.bottom = `${host.bottom - r.top + 10}px`;
    }
  }
  function closeMenus() { ctx.hidden = true; }
  document.addEventListener('mousedown', e => { if (!e.target.closest('.pmenu') && !e.target.closest('[data-menu]')) closeMenus(); });

  function modal(title) {
    const wrap = el(`<div class="vs-backdrop"><div class="vs-card" role="dialog"><div class="vs-head"><h2 class="vs-title">${esc(title)}</h2><button class="vs-x" aria-label="Close">${I.x}</button></div><div class="vs-body"></div></div></div>`);
    document.body.appendChild(wrap);
    const onKey = e => { if (e.key === 'Escape') { e.stopPropagation(); close(); } };
    window.addEventListener('keydown', onKey, true);
    const close = () => { wrap.classList.remove('open'); window.removeEventListener('keydown', onKey, true); setTimeout(() => wrap.remove(), 250); };
    wrap.addEventListener('click', e => { if (e.target === wrap) close(); });
    $('.vs-x', wrap).onclick = close;
    requestAnimationFrame(() => wrap.classList.add('open'));
    return { wrap, body: $('.vs-body', wrap), close };
  }
  let settingsModal = null;
  function openSettings() {
    if (settingsModal) { settingsModal.close(); settingsModal = null; return; }
    const m = settingsModal = modal('Settings');
    const q = v => v === 2160 ? '4K' : `${v}p`;
    const langs = [['en', 'English'], ['es', 'Spanish'], ['fr', 'French'], ['de', 'German'], ['pt', 'Portuguese'], ['it', 'Italian'], ['ja', 'Japanese'], ['ar', 'Arabic'], ['ru', 'Russian'], ['ko', 'Korean'], ['zh', 'Chinese'], ['tr', 'Turkish'], ['pl', 'Polish'], ['nl', 'Dutch'], ['sv', 'Swedish'], ['hi', 'Hindi']];
    const seg = (key, opts, fmt) => `<div class="vs-seg" data-seg="${key}">${opts.map(o => `<button data-v="${o}" class="${S[key] == o ? 'active' : ''}">${fmt ? fmt(o) : o}</button>`).join('')}</div>`;
    const sw = key => `<div class="vs-switch${S[key] ? ' on' : ''}"></div>`;
    const sel = (key, opts) => `<select class="vs-select" data-sel="${key}">${opts.map(([v, l]) => `<option value="${v}"${S[key] == v ? ' selected' : ''}>${l}</option>`).join('')}</select>`;
    const row = (label, hint, control, click) => `<div class="vs-row${click ? ' clickable' : ''}"${click ? ` data-click="${click}"` : ''}><div class="vs-row-text"><strong>${label}</strong>${hint ? `<small>${hint}</small>` : ''}</div>${control}</div>`;
    m.body.innerHTML = `
      <div class="vs-section"><div class="vs-label">Playback</div>
        ${row('Streaming quality', 'Highest rendition handed to the player.', seg('maxHeight', [480, 720, 1080, 2160], q))}
        ${row('Hardware decoding', 'Decodes on the GPU. Turn off if you see glitches.', sw('hwdec'), 'hwdec')}
        ${row('Auto-play next episode', 'Rolls into the next one with a short countdown.', sw('autoplay'), 'autoplay')}
        ${row('Auto-skip intros', 'Jumps past intros the moment they start.', sw('autoSkip'), 'autoSkip')}
        ${row('Skip buttons', 'Seconds for the arrows and the skip buttons.', seg('skipSeconds', [10, 15, 30], v => `${v}s`))}
      </div>
      <div class="vs-section"><div class="vs-label">Subtitles &amp; audio</div>
        ${row('Default audio (anime)', 'When a show offers both.', seg('dub', [false, true], v => v ? 'Dub' : 'Sub'))}
        ${row('Subtitle language', 'Picked automatically when a source has it.', sel('subtitleLang', langs))}
        ${row('Audio language', 'For sources with several audio tracks.', sel('audioLang', [['', 'Match subtitles']].concat(langs)))}
        ${row('Caption background', '', seg('subBg', ['shadow', 'box', 'none'], v => v[0].toUpperCase() + v.slice(1)))}
        ${row('Caption colour', '', seg('subColor', ['white', 'yellow', 'red', 'cyan'], v => v[0].toUpperCase() + v.slice(1)))}
      </div>
      <div class="vs-section"><div class="vs-label">Home &amp; downloads</div>
        ${row('Rotate the hero', 'Cycles the featured titles every few seconds.', sw('heroRotate'), 'heroRotate')}
        ${row('Download quality', '', seg('downloadHeight', [480, 720, 1080, 2160], q))}
        ${row('Downloads folder', esc(App.boot.downloads || ''), '<button class="vs-seg" style="padding:6px 12px;font-weight:600;font-size:12px;color:#fff" id="vs-folder">Open</button>')}
      </div>
      <div class="vs-section"><div class="vs-label">App</div>
        ${row('Discord presence', 'Shows what you are watching on your Discord profile.', sw('presence'), 'presence')}
        ${row('Keep running in the tray', 'Closing the window hides viel instead of quitting. Quit from the tray icon.', sw('tray'), 'tray')}
        ${row('Check for updates', 'Tells you when a newer build is on viel.tv.', sw('updates'), 'updates')}
        ${row('Media keys', 'Play/pause, next and previous track keys work anywhere.', '')}
      </div>
      <div class="vs-section"><div class="vs-label">Keyboard</div><div class="vs-keys">
        <div><span>Play / pause</span><span><kbd>Space</kbd> <kbd>K</kbd></span></div><div><span>Seek</span><span><kbd>←</kbd> <kbd>→</kbd></span></div>
        <div><span>Seek a minute</span><span><kbd>Shift</kbd>+<kbd>←</kbd>/<kbd>→</kbd></span></div><div><span>Volume</span><span><kbd>↑</kbd> <kbd>↓</kbd></span></div>
        <div><span>Mute</span><span><kbd>M</kbd></span></div><div><span>Fullscreen</span><span><kbd>F</kbd></span></div>
        <div><span>Subtitles</span><span><kbd>C</kbd></span></div><div><span>Next episode</span><span><kbd>N</kbd></span></div>
        <div><span>Speed</span><span><kbd>[</kbd> <kbd>]</kbd></span></div><div><span>Stats</span><span><kbd>I</kbd></span></div>
        <div><span>Episodes</span><span><kbd>E</kbd></span></div><div><span>Mini player</span><span><kbd>P</kbd></span></div>
        <div><span>Search</span><span><kbd>Ctrl</kbd>+<kbd>K</kbd></span></div><div><span>Back / close</span><span><kbd>Esc</kbd></span></div>
      </div></div>`;
    m.body.addEventListener('click', e => {
      const segBtn = e.target.closest('.vs-seg button[data-v]');
      if (segBtn) { const key = segBtn.closest('.vs-seg').dataset.seg; const raw = segBtn.dataset.v; const v = raw === 'true' ? true : raw === 'false' ? false : isNaN(Number(raw)) ? raw : Number(raw); Settings.set(key, v); $$('button', segBtn.parentElement).forEach(b => b.classList.toggle('active', b === segBtn)); afterSetting(key); return; }
      const rowEl = e.target.closest('.vs-row[data-click]');
      if (rowEl) { const key = rowEl.dataset.click; Settings.set(key, !S[key]); $('.vs-switch', rowEl).classList.toggle('on', S[key]); afterSetting(key); }
    });
    $$('select[data-sel]', m.body).forEach(s => s.onchange = () => { Settings.set(s.dataset.sel, s.value); afterSetting(s.dataset.sel); });
    byId('vs-folder').onclick = () => invoke('open_downloads');
    m.wrap.addEventListener('transitionend', () => { if (!m.wrap.classList.contains('open')) settingsModal = null; });
  }
  function afterSetting(key) {
    if (key === 'maxHeight' || key === 'subtitleLang' || key === 'audioLang') Settings.push();
    if (key === 'hwdec') Player.set('hwdec', S.hwdec ? 'auto-safe' : 'no');
    if (key === 'subBg' || key === 'subColor') Player.applySubStyle();
    if (key === 'skipSeconds') Player.refreshSkip();
    if (key === 'heroRotate') renderHero();
    if (key === 'tray') invoke('set_tray', { on: !!S.tray });
    if (key === 'presence') Player.presence();
  }

  let accountModal = null;
  function openAccount() {
    if (accountModal) { accountModal.close(); accountModal = null; return; }
    const m = accountModal = modal(App.user ? 'Account' : 'Sign in');
    if (App.user) {
      m.body.innerHTML = `<div class="vs-who"><h3>${esc(App.user)}</h3></div><p class="vs-since">Your list and watch history follow this account on every device, including the site and the iPhone app.</p>
        <dl class="vs-facts"><div class="vs-fact"><dt>Server</dt><dd><input id="acc-host" value="${esc(App.boot.host || '')}" spellcheck="false"></dd></div><div class="vs-fact"><dt>Version</dt><dd>${esc(App.boot.version || '')}</dd></div><div class="vs-fact"><dt>Player</dt><dd>${App.boot.mpv ? `<span style="color:#f87171">${esc(App.boot.mpv)}</span>` : 'libmpv ready'}</dd></div></dl>
        <div class="vs-foot"><a style="cursor:pointer;text-decoration:underline;text-underline-offset:3px" id="acc-site">Open viel.tv</a><button class="vs-signout" id="acc-out">Sign out</button></div>`;
      byId('acc-site').onclick = () => invoke('open_external', { url: App.boot.host || 'https://viel.tv' });
      byId('acc-host').onchange = async e => { await invoke('set_host', { host: e.target.value }); App.boot.host = e.target.value.trim() || 'https://viel.tv'; App.home = null; App.lists = {}; toast('Server updated'); };
      byId('acc-out').onclick = async () => { await api.post('logout').catch(() => {}); setUser(null); App.home = null; App.watchlist = null; App.history = null; m.close(); accountModal = null; openAccount(); };
    } else {
      let mode = 'login';
      m.body.innerHTML = `<div class="vs-section"><div class="vs-label">Welcome</div><div class="vs-auth-tabs"><button class="vs-auth-tab active" data-m="login">Sign in</button><button class="vs-auth-tab" data-m="register">Create account</button></div>
        <form id="auth-form" autocomplete="off"><input class="vs-input" name="username" placeholder="Username" autocomplete="username" required><input class="vs-input" name="password" type="password" placeholder="Password" autocomplete="current-password" required><button class="vs-btn" type="submit" id="auth-submit">Sign in</button><div class="vs-msg" id="auth-msg"></div>
        <div style="margin-top:14px;font-size:11.5px;color:rgba(255,255,255,.45);line-height:1.45;text-align:center">Same account as the website and the iPhone app.</div></form></div>`;
      $$('.vs-auth-tab', m.body).forEach(b => b.onclick = () => { mode = b.dataset.m; $$('.vs-auth-tab', m.body).forEach(x => x.classList.toggle('active', x === b)); byId('auth-submit').textContent = mode === 'login' ? 'Sign in' : 'Create account'; });
      byId('auth-form').onsubmit = async e => {
        e.preventDefault();
        const f = e.target, msg = byId('auth-msg');
        const username = f.username.value.trim(), password = f.password.value;
        if (!username || !password) { msg.className = 'vs-msg err'; msg.textContent = 'Fill in both fields.'; return; }
        byId('auth-submit').disabled = true;
        try { const r = await api.post(mode, { username, password }); setUser(r.username); App.home = null; App.watchlist = null; App.history = null; m.close(); accountModal = null; showMode('home'); }
        catch (err) { msg.className = 'vs-msg err'; msg.textContent = String(err); }
        byId('auth-submit').disabled = false;
      };
      setTimeout(() => { const i = $('input[name=username]', m.body); if (i) i.focus(); }, 60);
    }
    m.wrap.addEventListener('transitionend', () => { if (!m.wrap.classList.contains('open')) accountModal = null; });
  }

  const Player = (() => {
    const root = byId('player-root');
    const q = id => byId(id);
    const ui = {
      back: q('pl-back'), tbTitle: q('tb-title'), tbEp: q('tb-ep'), tbEpname: q('tb-epname'), infoBtn: q('title-info-btn'), listBtn: q('mylist-btn'),
      logo: q('show-logo'), showTitle: q('show-title-text'), pauseMeta: q('pause-meta'), pauseEp: q('pause-ep-title'), pauseDesc: q('pause-desc'),
      playpause: q('playpause'), iconPlay: q('icon-play'), iconPause: q('icon-pause'), rew: q('rew10'), ffw: q('fwd10'), mute: q('mute'), volume: q('volume'), time: q('time'),
      progress: q('progress-wrap'), play: q('progress-play'), buffer: q('progress-buffer'), thumb: q('progress-thumb'), seek: q('seek'), markers: q('seg-markers'),
      episodesToggle: q('episodes-toggle'), nextEpisode: q('next-episode'), pip: q('pip-btn'), fullscreen: q('fullscreen'),
      seekLeft: q('seek-left'), seekRight: q('seek-right'), burst: q('play-burst'), ring: q('buffer-ring'), mRew: q('m-rew'), mPlay: q('m-play'), mFwd: q('m-fwd'),
      skip: q('skip-intro'), nextOv: q('next-ep-overlay'), nextShow: q('next-ep-show'), nextTitle: q('next-ep-title'), nextBar: q('next-ep-bar'), nextCount: q('next-ep-countdown'), nextNow: q('next-ep-now'), nextCancel: q('next-ep-cancel'),
      endActions: q('end-actions'), endReplay: q('end-replay'), endNext: q('end-next'),
      scanning: q('scanning-overlay'), finderPoster: q('finder-poster'), finderKicker: q('finder-kicker'), finderTitle: q('finder-title'), finderEp: q('finder-ep'), scanMsg: q('scanning-msg'), finderSub: q('finder-sub'), finderFill: q('finder-bar-fill'), finderCount: q('finder-count'), finderSources: q('finder-sources'),
      toast: q('source-toast'), notifs: q('notif-stack'),
      settingsWrap: q('settings-panel-wrap'), settingsTitle: q('settings-title'), settingsBack: q('settings-back'),
      drawer: q('drawer'), lead: q('sheet-lead'), search: q('search-input'), seasonWrap: q('season-pill-wrap'), seasonBtn: q('season-pill-btn'), seasonLabel: q('season-pill-label'), seasonDrop: q('season-dropdown'), autonext: q('autonext-btn'), drawerClose: q('drawer-close'), bannerTitle: q('season-banner-title'), bannerDesc: q('season-banner-desc'), stack: q('episodes-stack'), drawerSheetClose: q('drawer-sheet-close'),
    };
    const st = { open: false, ready: false, initError: null, item: null, details: null, streams: [], streamIndex: -1, stream: null, candidates: [], loading: false, pos: 0, dur: 0, paused: true, subs: [], sid: null, audioTracks: null, lastSave: 0, lastSavedPos: -1, dismissedNext: false, skipped: new Set(), hideTimer: null, fullscreen: false, mini: false, scrubbing: false, props: {}, sleepAt: null, sleepAtEnd: false, statsOn: false, finishedAt: 0, ended: false, tab: null, drawerSeason: null, volumeBeforeMute: 100 };
    const cmd = (...args) => invoke('player_cmd', { args: args.map(String) }).catch(e => log('mpv', `${args.join(' ')} → ${e}`));
    const set = (name, value) => invoke('player_set', { name, value: String(value) }).catch(e => log('mpv', `set ${name} → ${e}`));
    const get = name => invoke('player_get', { name }).catch(() => null);
    const pad = n => String(n).padStart(2, '0');
    const tclock = s => { s = Math.max(0, Math.floor(s || 0)); const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), x = s % 60; return h ? `${h}:${pad(m)}:${pad(x)}` : `${pad(m)}:${pad(x)}`; };
    const SOURCE_FULL = { vixsrc: 'VixSrc', cno: 'Cino', mapple: 'Mapple', vidbolt: 'VidBolt', vidrock: 'VidRock', animekai: 'AnimeKai', animeparadise: 'AnimeParadise', luffy: 'LuffyTV', kototv: 'KotoTV', offline: 'Downloaded' };
    const fullName = s => SOURCE_FULL[s] || sourceLabel(s);

    async function init() {
      try {
        await invoke('player_init');
        st.ready = true;
        await set('volume', S.volume);
        await set('mute', S.mute ? 'yes' : 'no');
        await set('hwdec', S.hwdec ? 'auto-safe' : 'no');
        applySubStyle();
      } catch (e) { st.ready = false; st.initError = String(e); App.boot.mpv = String(e); log('mpv', `init failed: ${e}`); }
    }
    function applySubStyle() {
      if (!st.ready) return;
      set('sub-font-size', Math.round((S.subPx || 24) * 2));
      const bg = S.subBg || (S.subBackground === false ? 'shadow' : 'shadow');
      set('sub-back-color', bg === 'box' ? '#99000000' : '#00000000');
      set('sub-border-size', bg === 'none' ? 0 : bg === 'box' ? 1.2 : 2.6);
      set('sub-shadow-offset', bg === 'shadow' ? 2 : 0);
      set('sub-color', { white: '#FFFFFF', yellow: '#FDE047', red: '#F87171', cyan: '#67E8F9' }[S.subColor || 'white']);
      set('sub-margin-y', Math.round(60 + (S.subLift || 0) * 2));
    }
    function refreshSkip() { const n = S.skipSeconds; ui.rew.title = `-${n}s`; ui.ffw.title = `+${n}s`; }
    function applyScale() { ['s', 'm', 'l', 'xl'].forEach(k => root.classList.toggle(`ui-scale-${k}`, (S.uiScale || 'm') === k)); }

    function notify(text, ms) {
      const n = el(`<div class="notif-item">${esc(text)}</div>`);
      ui.notifs.appendChild(n);
      requestAnimationFrame(() => n.classList.add('notif-show'));
      setTimeout(() => { n.classList.remove('notif-show'); setTimeout(() => n.remove(), 400); }, ms || 2600);
    }
    let toastTimer2;
    function sourceToast(text) { ui.toast.textContent = text; ui.toast.classList.add('show'); clearTimeout(toastTimer2); toastTimer2 = setTimeout(() => ui.toast.classList.remove('show'), 2400); }

    function candidatesFor(item) {
      if (item.offline) return ['offline'];
      const e = Item.ep(item);
      if (Item.isAnime(item)) return (e && e.sources || []).map(s => s.provider).filter((v, i, a) => a.indexOf(v) === i);
      return Item.isMovie(item) ? ['vixsrc', 'cno', 'mapple', 'vidrock'] : ['vixsrc', 'cno', 'mapple', 'vidbolt', 'vidrock'];
    }
    function finder(item, state) {
      const e = Item.ep(item), anime = Item.isAnime(item);
      ui.finderPoster.src = item.title.poster || '';
      ui.finderKicker.textContent = [e ? (anime ? `E${numText(e.number)}` : `S${e.season} E${Math.trunc(e.number)}`) : kindLabel(item.title.kind), item.title.year].filter(Boolean).join(' · ');
      ui.finderTitle.textContent = item.title.name;
      ui.finderEp.textContent = e ? (e.name || '') : '';
      const names = st.candidates;
      ui.finderCount.textContent = names.length;
      ui.finderSources.innerHTML = names.map(n => { const s = state[n] || 'waiting'; return `<div class="fsrc ${s}" data-src="${esc(n)}"><span class="dot"></span><span><div class="n">${esc(fullName(n))}</div><div class="s">${s === 'checking' ? 'Checking' : s === 'found' ? 'Ready' : s === 'playing' ? 'Playing' : s === 'failed' ? 'No stream' : 'Waiting'}</div></span></div>`; }).join('');
      const done = names.filter(n => state[n] && state[n] !== 'checking' && state[n] !== 'waiting').length;
      ui.finderSub.textContent = names.length ? `${done} of ${names.length} checked` : '';
      ui.finderFill.style.width = `${names.length ? Math.round(done / names.length * 100) : 0}%`;
    }
    function scanning(on, msg) {
      root.classList.toggle('scanning', on);
      ui.scanning.classList.toggle('hidden', !on);
      if (msg) ui.scanMsg.textContent = msg;
    }

    async function start(item) {
      if (!st.ready) { toast(st.initError ? `Player unavailable: ${st.initError}` : 'Player is still starting'); return; }
      closeMenus();
      if (infoOv.classList.contains('open')) closeInfo();
      Object.assign(st, { item, streams: [], stream: null, streamIndex: -1, pos: item.startAt || 0, dur: 0, dismissedNext: false, skipped: new Set(), subs: [], sid: null, audioTracks: null, lastSavedPos: -1, finishedAt: 0, loading: true, ended: false, details: App.details[tkey(item.title)] || null });
      st.candidates = candidatesFor(item);
      ui.nextOv.classList.add('hidden'); ui.skip.classList.add('hidden'); ui.endActions.classList.add('hidden'); ui.markers.innerHTML = '';
      closeDrawer(); closeSettings();
      root.classList.toggle('is-movie', Item.isMovie(item));
      root.classList.toggle('is-anime', Item.isAnime(item));
      root.classList.toggle('multi-season', Item.isAnime(item) ? (item.chain || []).length > 1 : new Set(item.episodes.map(e => e.season)).size > 1);
      if (!st.open) {
        st.open = true;
        root.hidden = false;
        applyScale();
        document.documentElement.classList.add('video');
        invoke('player_show', { visible: true });
        window.addEventListener('keydown', onKey, true);
      }
      renderTitle();
      const e = Item.ep(item);
      const state = {}; st.candidates.forEach(n => { state[n] = 'checking'; });
      finder(item, state);
      scanning(true, e ? `Loading episode ${numText(e.number)}…` : 'Finding a source…');
      showControls();
      updateTime();
      await cmd('stop');
      let reply;
      try { reply = item.offline ? await api.get('offline', { key: item.offline }) : await api.post('play', Item.request(item)); }
      catch (err) { st.candidates.forEach(n => { state[n] = 'failed'; }); finder(item, state); scanning(true, `Couldn't resolve this one. ${err}`); st.loading = false; return; }
      if (st.item !== item) return;
      let streams = reply.streams || [];
      const pref = Memory.preferredSource(item.title);
      if (pref) streams = streams.slice().sort((a, b) => (a.source === pref ? 0 : 1) - (b.source === pref ? 0 : 1));
      st.streams = streams;
      st.candidates.forEach(n => { state[n] = streams.some(s => s.source === n) ? 'found' : 'failed'; });
      streams.forEach(s => { if (!st.candidates.includes(s.source)) { st.candidates.push(s.source); state[s.source] = 'found'; } });
      st.finderState = state;
      finder(item, state);
      if (!streams.length) { scanning(true, 'No source has this one right now.'); st.loading = false; return; }
      loadStream(0);
    }
    async function loadStream(i, resumeAt) {
      const s = st.streams[i];
      if (!s) { scanning(true, 'Every source failed for this one.'); st.loading = false; return; }
      Object.assign(st, { streamIndex: i, stream: s, loading: true, subs: s.subs || [], sid: null, ended: false });
      ui.endActions.classList.add('hidden');
      const at = resumeAt != null ? resumeAt : (st.item.startAt || null);
      if (st.finderState) { Object.keys(st.finderState).forEach(k => { if (st.finderState[k] === 'playing') st.finderState[k] = 'found'; }); st.finderState[s.source] = 'playing'; finder(st.item, st.finderState); }
      scanning(true, i > 0 ? `Trying ${fullName(s.source)}…` : 'Loading…');
      ui.markers.innerHTML = '';
      await set('start', at && at > 0 ? String(at) : 'none');
      await set('pause', 'no');
      await cmd('loadfile', s.url, 'replace');
      renderSettingsPane();
      log('play', `${Item.heading(st.item)} via ${s.source}${at ? ` @${Math.round(at)}` : ''}`);
    }
    function presence() {
      const it = st.item;
      if (!S.presence || !st.open || !it || st.loading || (st.stream && st.stream.source === 'offline')) { invoke('presence', { playing: null }).catch(() => {}); return; }
      const e = Item.ep(it);
      invoke('presence', { playing: { kind: it.title.kind, id: it.title.id, show: it.title.name, episode_title: e ? e.name || '' : '', season: e ? e.season : 0, episode: e ? e.number : 0, position: st.pos || 0, paused: !!st.paused, poster: it.title.poster || '', host: App.boot.host || '' } }).catch(() => {});
    }
    let presenceTimer = null;
    async function onLoaded() {
      st.loading = false;
      scanning(false);
      clearInterval(presenceTimer);
      presenceTimer = setInterval(presence, 15000);
      presence();
      if (st.stream && st.stream.source !== 'offline') { Memory.rememberSource(st.item.title, st.stream.source); sourceToast(`Playing from ${fullName(st.stream.source)}`); }
      await set('start', 'none');
      const want = Memory.subtitle(st.item.title) || S.subtitleLang;
      let pick = -1;
      for (let i = 0; i < st.subs.length; i++) {
        const sub = st.subs[i];
        await cmd('sub-add', sub.url, 'auto', langName(sub.lang), sub.lang);
        if (pick < 0 && want !== 'off' && langMatch(sub.lang, want)) pick = i;
      }
      await selectSub(pick >= 0 ? pick : null, true);
      if (st.stream && st.stream.hls && st.stream.source !== 'offline') api.get('audio', { url: st.stream.url }).then(rows => { st.audioTracks = rows || []; renderSettingsPane(); }).catch(() => { st.audioTracks = []; });
      drawMarkers();
      renderSettingsPane();
      scheduleHide();
    }
    function drawMarkers() {
      if (!st.dur || !st.stream) { ui.markers.innerHTML = ''; return; }
      ui.markers.innerHTML = ['intro', 'outro'].map(k => { const m = st.stream[k]; return m && m.length === 2 ? `<div class="seg-mark ${k === 'intro' ? 'seg-intro' : 'seg-credits'}" style="left:${m[0] / st.dur * 100}%;width:${(m[1] - m[0]) / st.dur * 100}%"></div>` : ''; }).join('');
    }
    async function selectSub(index, silent) {
      st.sid = index;
      if (index == null) await set('sid', 'no');
      else {
        let tracks = [];
        try { tracks = JSON.parse(await get('track-list')) || []; } catch (_) {}
        const external = tracks.filter(t => t.type === 'sub' && t.external);
        await set('sid', external[index] ? external[index].id : index + 1);
        await set('sub-visibility', 'yes');
      }
      if (!silent) { Memory.rememberSubtitle(st.item.title, index == null ? null : st.subs[index].lang); notify(index == null ? 'Subtitles off' : `Subtitles: ${langName(st.subs[index].lang)}`); }
      renderSettingsPane();
    }
    function renderTitle() {
      const it = st.item, e = Item.ep(it), anime = Item.isAnime(it);
      ui.tbTitle.textContent = it.title.name;
      ui.tbEp.textContent = e ? (anime ? `E${numText(e.number)}` : `S${e.season} E${Math.trunc(e.number)}`) + (it.dub ? ' · Dub' : '') : '';
      ui.tbEpname.textContent = e ? (e.name || '') : '';
      ui.listBtn.classList.toggle('on', inList(it.title));
      ui.showTitle.textContent = it.title.name;
      ui.showTitle.style.display = 'block';
      ui.logo.style.display = 'none';
      ui.pauseEp.textContent = e ? (e.name || `Episode ${numText(e.number)}`) : '';
      ui.pauseDesc.textContent = e ? (e.overview || '') : (it.title.overview || '');
      ui.pauseMeta.textContent = e ? [`Episode ${numText(e.number)}`, e.runtime ? human(e.runtime) : ''].filter(Boolean).join(' · ') : [it.title.year, kindLabel(it.title.kind)].filter(Boolean).join(' · ');
      ui.lead.textContent = it.title.name;
      ui.bannerTitle.textContent = it.title.name;
      detailsFor(it.title).then(d => {
        if (!d || st.item !== it) return;
        st.details = d;
        if (d.logo) { ui.logo.src = d.logo; ui.logo.style.display = 'block'; ui.showTitle.style.display = 'none'; }
        if (e) ui.pauseMeta.textContent = [`Episode ${numText(e.number)}`, (e.runtime || d.runtime) ? human(e.runtime || d.runtime) : ''].filter(Boolean).join(' · ');
        else { ui.pauseMeta.textContent = [d.year, d.runtime ? human(d.runtime) : '', (d.genres || []).slice(0, 2).join(' · ')].filter(Boolean).join(' · '); if (!ui.pauseDesc.textContent) ui.pauseDesc.textContent = d.overview || ''; }
        ui.bannerDesc.textContent = d.overview || '';
      });
    }
    function updateTime() {
      ui.time.textContent = `${tclock(st.pos)} / ${tclock(st.dur)}`;
      if (!st.scrubbing) { const f = st.dur ? st.pos / st.dur : 0; ui.play.style.width = `${f * 100}%`; ui.thumb.style.left = `${f * 100}%`; ui.seek.value = f * 100; }
      const cache = st.props['demuxer-cache-duration'] || 0;
      ui.buffer.style.width = st.dur ? `${clamp((st.pos + cache) / st.dur, 0, 1) * 100}%` : '0%';
    }
    function tick() {
      if (!st.open || !st.item) return;
      if (st.dur > 0 && !st.loading && st.pos > 0 && Date.now() - st.lastSave > 5000 && Math.abs(st.pos - st.lastSavedPos) > 3) save(false);
      checkMarkers(); checkNext();
      if (st.statsOn) renderStats();
      if (st.sleepAt && Date.now() >= st.sleepAt) { st.sleepAt = null; close(); }
    }
    function save(final) {
      if (!st.item || !(st.dur > 0) || st.item.offline) return;
      st.lastSave = Date.now(); st.lastSavedPos = st.pos;
      const p = Item.progress(st.item, final ? st.dur : st.pos, st.dur);
      Memory.remember(p);
      api.post('progress', p).catch(e => log('progress', String(e)));
    }
    function checkMarkers() {
      const s = st.stream;
      if (!s) { ui.skip.classList.add('hidden'); return; }
      let which = null;
      for (const k of ['intro', 'outro']) { const m = s[k]; if (m && m.length === 2 && !st.skipped.has(k) && st.pos >= m[0] && st.pos < m[1] - 1) { which = k; break; } }
      if (!which) { ui.skip.classList.add('hidden'); return; }
      if (S.autoSkip && which === 'intro') { st.skipped.add(which); seek(s[which][1], true); notify('Skipped intro'); return; }
      if (ui.skip.classList.contains('hidden') || ui.skip.dataset.which !== which) {
        ui.skip.dataset.which = which;
        ui.skip.textContent = which === 'intro' ? 'Skip Intro ▶' : 'Skip Outro ▶';
        ui.skip.classList.remove('hidden');
        ui.skip.onclick = () => { st.skipped.add(which); seek(s[which][1], true); ui.skip.classList.add('hidden'); };
      }
    }
    function checkNext() {
      const next = st.item && Item.next(st.item);
      const left = st.dur - st.pos;
      const show = next && S.autoplay && !st.dismissedNext && st.dur > 0 && !st.loading && left <= 20 && left > 0;
      if (!show) { ui.nextOv.classList.add('hidden'); return; }
      const e = Item.ep(next);
      if (ui.nextOv.classList.contains('hidden')) {
        ui.nextShow.textContent = next.title.name;
        ui.nextTitle.textContent = epLabel(e, Item.isAnime(next));
        ui.nextOv.classList.remove('hidden');
      }
      ui.nextCount.textContent = Math.ceil(left);
      ui.nextBar.style.width = `${clamp((20 - left) / 20, 0, 1) * 100}%`;
    }
    function goNext() {
      const next = st.item && Item.next(st.item);
      if (!next) return;
      save(true);
      api.post('progress', Item.progress(next, 0, st.dur || 0)).catch(() => {});
      ui.nextOv.classList.add('hidden');
      start(next);
    }
    function finished() {
      if (st.ended) return;
      st.ended = true;
      save(true);
      const next = Item.next(st.item);
      if (next && !st.item.offline) api.post('progress', Item.progress(next, 0, st.dur || 0)).catch(() => {});
      if (st.sleepAtEnd) { close(); return; }
      if (next && S.autoplay && !st.dismissedNext && !st.item.offline) { start(next); return; }
      ui.nextOv.classList.add('hidden');
      ui.endActions.classList.remove('hidden');
      ui.endNext.style.display = next ? '' : 'none';
      showControls();
    }
    async function close() {
      if (!st.open) return;
      save(false);
      st.open = false;
      st.sleepAt = null; st.sleepAtEnd = false;
      window.removeEventListener('keydown', onKey, true);
      await cmd('stop');
      clearInterval(presenceTimer);
      invoke('presence', { playing: null }).catch(() => {});
      invoke('player_show', { visible: false });
      if (st.fullscreen) { st.fullscreen = false; invoke('win', { action: 'fullscreen', on: false }); }
      if (st.mini) toggleMini(false);
      root.hidden = true;
      root.classList.remove('scanning', 'buffering', 'paused', 'show-controls', 'drawer-open', 'settings-open');
      ui.scanning.classList.add('hidden'); ui.nextOv.classList.add('hidden'); ui.skip.classList.add('hidden'); ui.endActions.classList.add('hidden');
      closeDrawer(); closeSettings();
      document.documentElement.classList.remove('video');
      closeMenus();
      App.homeAt = 0; App.historyAt = 0;
      if (App.page === 'history') renderHistory();
      else if (!App.page && App.mode === 'home') { App.home = null; showMode('home'); }
    }
    function burst(pausing) {
      ui.burst.classList.remove('go', 'p', 'q');
      void ui.burst.offsetWidth;
      ui.burst.classList.add(pausing ? 'q' : 'p', 'go');
    }
    function seekFlash(right, n) {
      const f = right ? ui.seekRight : ui.seekLeft;
      f.querySelector('span').textContent = `${n}s`;
      f.classList.remove('go'); void f.offsetWidth; f.classList.add('go');
    }
    function togglePlay() {
      if (!st.item || st.loading) return;
      if (st.ended) { seek(0, true); set('pause', 'no'); st.ended = false; ui.endActions.classList.add('hidden'); return; }
      const toPause = !st.paused;
      set('pause', toPause ? 'yes' : 'no');
      burst(toPause);
    }
    function seek(to, absolute) {
      if (!st.dur) return;
      const target = clamp(absolute ? to : st.pos + to, 0, Math.max(st.dur - 0.5, 0));
      st.pos = target; updateTime();
      cmd('seek', target, 'absolute+exact');
      if (!absolute) seekFlash(to > 0, Math.abs(to));
      if (st.ended && target < st.dur - 1) { st.ended = false; ui.endActions.classList.add('hidden'); set('pause', 'no'); }
      showControls();
    }
    function paintVolume() {
      const v = S.mute ? 0 : Math.min(S.volume, 100) / 100;
      ui.volume.value = v;
      ui.volume.style.background = `linear-gradient(to right, var(--accent-hi,#f87171) ${v * 100}%, rgba(255,255,255,0.25) ${v * 100}%)`;
      ui.mute.innerHTML = S.mute || S.volume === 0 ? I.volx : I.vol;
    }
    function setVolume(v, quiet) {
      v = clamp(Math.round(v), 0, 150);
      Settings.set('volume', v); set('volume', v);
      if (S.mute && v > 0) { Settings.set('mute', false); set('mute', 'no'); }
      paintVolume();
      if (!quiet) notify(`Volume ${v}%`, 1200);
    }
    function toggleMute() { Settings.set('mute', !S.mute); set('mute', S.mute ? 'yes' : 'no'); paintVolume(); notify(S.mute ? 'Muted' : `Volume ${S.volume}%`, 1200); }
    function setSpeed(v) { v = clamp(Math.round(v * 100) / 100, 0.25, 4); Settings.set('speed', v); set('speed', v); notify(`${v}× speed`, 1200); renderSettingsPane(); }
    function toggleFullscreen(on) { st.fullscreen = on == null ? !st.fullscreen : on; invoke('win', { action: 'fullscreen', on: st.fullscreen }); }
    function toggleMini(on) { st.mini = on == null ? !st.mini : on; invoke('win', { action: 'mini', on: st.mini }); root.classList.toggle('mini', st.mini); }
    function showControls() { root.classList.add('show-controls'); clearTimeout(st.hideTimer); if (!st.paused && !st.loading) scheduleHide(); }
    function scheduleHide() { clearTimeout(st.hideTimer); st.hideTimer = setTimeout(() => { if (!st.paused && !st.scrubbing && !root.classList.contains('drawer-open') && !root.classList.contains('settings-open')) root.classList.remove('show-controls'); }, 3000); }
    let statsEl = null;
    function renderStats() {
      const p = st.props, w = p['video-params/w'], h = p['video-params/h'];
      if (!statsEl) { statsEl = el('<div class="stats"></div>'); root.appendChild(statsEl); }
      statsEl.hidden = !st.statsOn;
      const lines = [
        ['Source', `${fullName(st.stream && st.stream.source)}${st.stream && st.stream.hls ? ' · HLS' : ''}`],
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
      statsEl.textContent = lines.map(([k, v]) => `${k.padEnd(9)}${v}`).join('\n');
    }
    function onProp(name, value) {
      st.props[name] = value;
      switch (name) {
        case 'pause': st.paused = !!value; ui.iconPlay.style.display = st.paused ? '' : 'none'; ui.iconPause.style.display = st.paused ? 'none' : ''; root.classList.toggle('paused', st.paused); if (st.paused) showControls(); else scheduleHide(); if (!st.loading) presence(); break;
        case 'time-pos': if (value != null && !st.scrubbing) { st.pos = value; updateTime(); } break;
        case 'duration': if (value != null) { st.dur = value; updateTime(); drawMarkers(); } break;
        case 'paused-for-cache': root.classList.toggle('buffering', !!value && !st.loading); ui.ring.classList.toggle('hidden', !(value && !st.loading)); break;
        case 'eof-reached': if (value && st.dur > 0 && !st.loading) finished(); break;
        case 'volume': if (value != null) { S.volume = Math.round(value); paintVolume(); } break;
        case 'demuxer-cache-duration': updateTime(); break;
      }
    }
    function onEvent(p) {
      switch (p.event) {
        case 'loaded': onLoaded(); break;
        case 'end':
          if (!st.open) break;
          if (p.reason === 'error') {
            log('play', `${st.stream ? st.stream.source : '?'} failed: ${p.error}`);
            if (st.finderState && st.stream) { st.finderState[st.stream.source] = 'failed'; }
            if (st.streams[st.streamIndex + 1]) { sourceToast(`${fullName(st.stream.source)} failed, trying ${fullName(st.streams[st.streamIndex + 1].source)}`); loadStream(st.streamIndex + 1, st.pos > 5 ? st.pos : null); }
            else { finder(st.item, st.finderState || {}); scanning(true, `Couldn't play this one (${p.error}).`); st.loading = false; }
          } else if (p.reason === 'eof' && st.dur > 0 && !st.loading) finished();
          break;
        case 'log': if (p.level === 'error' || p.level === 'fatal') log('mpv', `${p.prefix}: ${p.text}`); break;
        case 'prop': onProp(p.name, p.value); break;
      }
    }

    function openDrawer() {
      if (!st.item || Item.isMovie(st.item) || !st.item.episodes.length) return;
      closeSettings();
      renderDrawer();
      ui.drawer.classList.add('open'); ui.drawer.setAttribute('aria-hidden', 'false');
      root.classList.add('drawer-open');
      setTimeout(() => { const on = $('.ep-tile.active', ui.stack); if (on) on.scrollIntoView({ inline: 'center', block: 'nearest' }); }, 50);
    }
    function closeDrawer() { ui.drawer.classList.remove('open'); ui.drawer.setAttribute('aria-hidden', 'true'); root.classList.remove('drawer-open'); ui.seasonDrop.classList.remove('open'); }
    function toggleDrawer() { if (root.classList.contains('drawer-open')) closeDrawer(); else openDrawer(); }
    function renderDrawer() {
      const it = st.item, anime = Item.isAnime(it), cur = Item.ep(it);
      const seasons = anime ? [] : Array.from(new Set(it.episodes.map(e => e.season)));
      if (st.drawerSeason == null || (!anime && !seasons.includes(st.drawerSeason))) st.drawerSeason = cur ? cur.season : seasons[0];
      ui.autonext.classList.toggle('on', !!S.autoplay);
      if (anime) {
        const chain = it.chain || [];
        ui.seasonLabel.textContent = (chain.find(c => c.id === it.title.id) || {}).title || it.title.name;
        ui.seasonDrop.innerHTML = chain.map((c, i) => `<div class="item${c.id === it.title.id ? ' active' : ''}" data-a="${c.id}">${esc(c.title || `Season ${c.season}`)}</div>`).join('');
      } else {
        ui.seasonLabel.textContent = st.drawerSeason === 0 ? 'Specials' : `S${st.drawerSeason}`;
        ui.seasonDrop.innerHTML = seasons.map(s => `<div class="item${s === st.drawerSeason ? ' active' : ''}" data-s="${s}">${s === 0 ? 'Specials' : `Season ${s}`}</div>`).join('');
      }
      const needle = (ui.search.value || '').trim().toLowerCase();
      const watched = Memory.watched(tkey(it.title));
      ui.stack.innerHTML = it.episodes.map((e, i) => {
        if (!anime && e.season !== st.drawerSeason) return '';
        if (needle && !(String(e.number).includes(needle) || (e.name || '').toLowerCase().includes(needle))) return '';
        const active = i === it.index;
        const seen = watched.has(`${e.season}-${Math.trunc(e.number)}`);
        return `<a class="ep-tile${active ? ' active playing' : ''}" data-i="${i}"><span class="th">${(e.still || it.title.poster) ? `<img loading="lazy" decoding="async" src="${esc(e.still || it.title.poster)}" alt="">` : ''}<span class="num">E${esc(numText(e.number))}</span><span class="now">NOW PLAYING</span><span class="tile-acts"><button type="button" class="save${inList(it.title) ? ' on' : ''}" data-act="save" aria-label="Save to My List"><svg viewBox="0 0 24 24"><path d="M6 4h12a1 1 0 0 1 1 1v15.4a.6.6 0 0 1-.92.5L12 17.5l-6.08 3.9A.6.6 0 0 1 5 20.9V5a1 1 0 0 1 1-1z"></path></svg></button><button type="button" class="seen${seen ? ' on' : ''}" data-act="seen" aria-label="Mark watched"><svg viewBox="0 0 24 24"><path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12z"></path><circle cx="12" cy="12" r="3"></circle></svg></button></span></span><span class="body"><span class="tt">${esc(e.name || `Episode ${numText(e.number)}`)}</span><p class="desc">${esc(e.overview || '')}</p><span class="rm">${e.runtime ? esc(human(e.runtime)) : ''}</span></span></a>`;
      }).join('') || '<div style="padding:20px;color:var(--fg-mid)">No episodes match.</div>';
    }
    ui.stack.addEventListener('click', e => {
      const act = e.target.closest('[data-act]');
      const tile = e.target.closest('.ep-tile');
      if (!tile) return;
      const i = Number(tile.dataset.i), ep = st.item.episodes[i];
      if (act) {
        e.preventDefault(); e.stopPropagation();
        if (act.dataset.act === 'save') { toggleList(st.item.title).then(() => renderDrawer()); }
        else { const key = tkey(st.item.title), k = `${ep.season}-${Math.trunc(ep.number)}`; const m = Memory.map('watched'); const s = new Set(m[key] || []); if (s.has(k)) s.delete(k); else s.add(k); m[key] = Array.from(s); Memory.store('watched', m); act.classList.toggle('on'); }
        return;
      }
      if (i !== st.item.index) { closeDrawer(); start(Item.at(st.item, i)); } else closeDrawer();
    });
    ui.seasonBtn.onclick = e => { e.stopPropagation(); ui.seasonDrop.classList.toggle('open'); };
    ui.seasonDrop.addEventListener('click', async e => {
      const item = e.target.closest('.item'); if (!item) return;
      ui.seasonDrop.classList.remove('open');
      if (item.dataset.s != null) { st.drawerSeason = Number(item.dataset.s); renderDrawer(); return; }
      const id = Number(item.dataset.a);
      if (id === st.item.title.id) return;
      try {
        const [d, list] = await Promise.all([api.get('details', { kind: 'anime', id: String(id) }), api.get('title', { kind: 'anime', id: String(id), dub: st.item.dub ? '1' : '0' })]);
        App.details[`anime-${id}`] = d;
        const title = remember({ id: d.id, kind: 'anime', name: d.name, year: d.year, rating: d.rating, overview: d.overview, poster: d.poster, backdrop: d.backdrop, genres: d.genres, seasons: d.seasons });
        if (!list.episodes.length) { notify('No episodes listed for that season'); return; }
        closeDrawer();
        start(Item.make(title, list.episodes, 0, list.dub, null, d.seasons || []));
      } catch (err) { notify(String(err)); }
    });
    ui.search.addEventListener('input', () => renderDrawer());
    ui.autonext.onclick = () => { Settings.set('autoplay', !S.autoplay); ui.autonext.classList.toggle('on', S.autoplay); };
    ui.drawerClose.onclick = closeDrawer; ui.drawerSheetClose.onclick = closeDrawer;

    const TABS = ['quality', 'subtitles', 'audio', 'servers', 'download', 'speed', 'display'];
    const TAB_TITLES = { home: 'Settings', quality: 'Quality', subtitles: 'Subtitles', audio: 'Audio', servers: 'Servers', download: 'Download', speed: 'Speed', display: 'Display' };
    function openSettings(tab) {
      closeDrawer();
      st.tab = tab || 'home';
      ui.settingsWrap.classList.add('open');
      root.classList.add('settings-open');
      renderSettingsPane();
    }
    function closeSettings() { ui.settingsWrap.classList.remove('open'); root.classList.remove('settings-open'); st.tab = null; }
    function toggleSettings(tab) { if (root.classList.contains('settings-open') && (st.tab === tab || !tab)) closeSettings(); else openSettings(tab); }
    const card = (label, badge, active, attrs, soft) => `<div class="opt-card${active ? ' active' : ''}" ${attrs || ''}><span>${esc(label)}</span>${badge ? `<span class="${soft ? 'badge-soft' : 'badge'}">${esc(badge)}</span>` : ''}${active ? '<span class="check-dot"></span>' : ''}</div>`;
    const leadCard = (title, sub, active, attrs) => `<div class="opt-card${active ? ' active' : ''}" ${attrs || ''}><span class="lead"><span class="lead-text"><strong>${esc(title)}</strong><small>${esc(sub)}</small></span></span>${active ? '<span class="check-dot"></span>' : ''}</div>`;
    function renderSettingsPane() {
      if (!root.classList.contains('settings-open')) return;
      const tab = st.tab || 'home';
      $$('.settings-tabs .tab', ui.settingsWrap).forEach(b => b.classList.toggle('active', b.dataset.tab === tab));
      $$('.settings-pane', ui.settingsWrap).forEach(p => p.classList.toggle('active', p.dataset.pane === tab));
      ui.settingsTitle.textContent = TAB_TITLES[tab] || 'Settings';
      ui.settingsBack.style.visibility = tab === 'home' ? 'hidden' : 'visible';
      const pane = byId(`pane-${tab}`);
      if (!pane) return;
      const it = st.item;
      if (tab === 'home') {
        pane.innerHTML = [
          leadCard('Quality', S.maxHeight === 2160 ? '4K' : `${S.maxHeight}p`, false, 'data-go="quality"'),
          leadCard('Subtitles', st.sid == null ? 'Off' : (st.subs[st.sid] ? langName(st.subs[st.sid].lang) : 'On'), false, 'data-go="subtitles"'),
          leadCard('Audio', it && Item.isAnime(it) ? (it.dub ? 'Dub' : 'Sub') : (S.audioLang ? langName(S.audioLang) : 'Default'), false, 'data-go="audio"'),
          leadCard('Servers', st.stream ? fullName(st.stream.source) : '—', false, 'data-go="servers"'),
          leadCard('Speed', `${S.speed || 1}×`, false, 'data-go="speed"'),
          leadCard('Display', 'Captions, size, sleep timer', false, 'data-go="display"'),
          it && !it.offline ? leadCard('Download', Item.isMovie(it) ? 'Save this movie offline' : 'Save this episode offline', false, 'data-go="download"') : '',
        ].join('');
      } else if (tab === 'quality') {
        pane.innerHTML = [2160, 1080, 720, 480].map(h => card(h === 2160 ? '4K' : `${h}p`, h === 2160 ? 'UHD' : h === 1080 ? 'HD' : h === 720 ? 'HD' : 'SD', S.maxHeight === h, `data-h="${h}"`)).join('');
      } else if (tab === 'subtitles') {
        const internal = (st.props['track-list'] || []).filter(t => t.type === 'sub' && !t.external);
        pane.innerHTML = card('Off', 'No Subtitles', st.sid == null, 'data-sub="off"', true) + st.subs.map((s, i) => card(langName(s.lang), 'Built-in', st.sid === i, `data-sub="${i}"`)).join('') + internal.map(t => card(t.title || langName(t.lang) || `Track ${t.id}`, 'Embedded', t.selected && st.sid === -1, `data-tid="${t.id}"`)).join('');
      } else if (tab === 'audio') {
        const internal = (st.props['track-list'] || []).filter(t => t.type === 'audio');
        let html = '';
        if (it && Item.isAnime(it)) html += card('Subbed (Japanese audio)', '', !it.dub, 'data-cat="sub"') + card('Dubbed (English audio)', '', it.dub, 'data-cat="dub"');
        if (internal.length > 1) html += internal.map(t => card(t.title || langName(t.lang) || `Track ${t.id}`, t.lang || '', !!t.selected, `data-aid="${t.id}"`)).join('');
        else if (st.audioTracks && st.audioTracks.length) html += st.audioTracks.map(t => card(t.name || langName(t.lang), t.lang || '', S.audioLang ? langMatch(t.lang, S.audioLang) : !!t.default, `data-alang="${esc(t.lang)}"`)).join('');
        pane.innerHTML = html || card('Only one audio track', '', true);
      } else if (tab === 'servers') {
        pane.innerHTML = st.streams.map((s, i) => leadCard(fullName(s.source), [s.hls ? 'HLS' : 'MP4', s.subs && s.subs.length ? `${s.subs.length} subtitle${s.subs.length > 1 ? 's' : ''}` : '', i === st.streamIndex ? 'playing' : ''].filter(Boolean).join(' · '), i === st.streamIndex, `data-srv="${i}"`)).join('') || card('No other sources', '', true);
      } else if (tab === 'download') {
        pane.innerHTML = it ? leadCard(Item.isMovie(it) ? 'Download this movie' : 'Download this episode', `${S.downloadHeight === 2160 ? '4K' : `${S.downloadHeight}p`} · saved to this PC`, false, 'data-dl="1"') + [2160, 1080, 720, 480].map(h => card(h === 2160 ? '4K' : `${h}p`, 'download quality', S.downloadHeight === h, `data-dlh="${h}"`)).join('') : '';
      } else if (tab === 'speed') {
        pane.innerHTML = [0.5, 0.75, 1, 1.25, 1.5, 2].map(v => card(v === 1 ? '1x (Normal)' : `${v}x`, '', (S.speed || 1) === v, `data-speed="${v}"`)).join('');
      } else if (tab === 'display') {
        const segs = (key, opts, cur) => `<div style="display:flex;gap:4px;flex-wrap:wrap" data-seg="${key}">${opts.map(([v, l]) => `<button data-v="${v}" class="${cur === v ? 'active' : ''}" style="flex:1;min-width:44px;padding:7px 10px;border-radius:8px;border:1px solid rgba(255,255,255,0.08);background:${cur === v ? 'rgba(220,38,38,0.18)' : 'rgba(255,255,255,0.04)'};color:${cur === v ? '#f87171' : '#fff'};font:inherit;font-size:11.5px;font-weight:600;cursor:pointer;letter-spacing:.02em">${l}</button>`).join('')}</div>`;
        const block = (title, inner) => `<div class="opt-card" style="flex-direction:column;align-items:stretch;gap:8px"><div style="font-size:12.5px;font-weight:600">${title}</div>${inner}</div>`;
        const sleep = st.sleepAtEnd ? 'end' : st.sleepAt ? 'on' : 'off';
        pane.innerHTML = block('Player UI size', segs('uiScale', [['s', 'S'], ['m', 'M'], ['l', 'L'], ['xl', 'XL']], S.uiScale || 'm'))
          + `<div class="opt-card" style="flex-direction:column;align-items:stretch;gap:8px"><div style="display:flex;justify-content:space-between;align-items:baseline"><span style="font-size:12.5px;font-weight:600">Caption size</span><span id="vsd-size-val" style="font-size:11px;color:rgba(255,255,255,0.5);font-variant-numeric:tabular-nums">${S.subPx || 24}px</span></div><input id="vsd-size-slider" type="range" min="14" max="56" step="1" value="${S.subPx || 24}" style="width:100%;appearance:none;height:4px;border-radius:2px;background:rgba(255,255,255,0.16);outline:0"></div>`
          + block('Caption background', segs('subBg', [['shadow', 'Shadow'], ['box', 'Box'], ['none', 'None']], S.subBg || 'shadow'))
          + block('Caption colour', segs('subColor', [['white', 'White'], ['yellow', 'Yellow'], ['red', 'Red'], ['cyan', 'Cyan']], S.subColor || 'white'))
          + block('Caption position', segs('subLift', [[0, 'Bottom'], [40, 'Raised'], [120, 'Middle']], S.subLift || 0))
          + block('Sleep timer', segs('sleep', [['off', 'Off'], [15, '15m'], [30, '30m'], [60, '60m'], ['end', 'End of episode']], sleep === 'end' ? 'end' : sleep === 'on' ? null : 'off'))
          + card('Auto-play next episode', S.autoplay ? 'On' : 'Off', !!S.autoplay, 'data-toggle="autoplay"', true)
          + card('Auto-skip intros', S.autoSkip ? 'On' : 'Off', !!S.autoSkip, 'data-toggle="autoSkip"', true)
          + card('Stats for nerds', st.statsOn ? 'On' : 'Off', st.statsOn, 'data-toggle="stats"', true);
      }
    }
    ui.settingsWrap.addEventListener('click', async e => {
      if (e.target.closest('.settings-close, .menu-close, .sheet-close')) { closeSettings(); return; }
      if (e.target.closest('.menu-back')) { st.tab = 'home'; renderSettingsPane(); return; }
      const tabBtn = e.target.closest('.settings-tabs .tab'); if (tabBtn) { st.tab = tabBtn.dataset.tab; renderSettingsPane(); return; }
      const seg = e.target.closest('[data-seg] button[data-v]');
      if (seg) {
        const key = seg.parentElement.dataset.seg; const raw = seg.dataset.v; const v = isNaN(Number(raw)) ? raw : Number(raw);
        if (key === 'sleep') { st.sleepAtEnd = v === 'end'; st.sleepAt = typeof v === 'number' ? Date.now() + v * 60000 : null; notify(v === 'end' ? 'Stops after this episode' : typeof v === 'number' ? `Sleep in ${v} min` : 'Sleep timer off'); }
        else { Settings.set(key, v); if (key === 'uiScale') applyScale(); else applySubStyle(); }
        renderSettingsPane(); return;
      }
      const c = e.target.closest('.opt-card'); if (!c) return;
      const d = c.dataset;
      if (d.go) { st.tab = d.go; renderSettingsPane(); return; }
      if (d.h) { Settings.set('maxHeight', Number(d.h)); await Settings.push(); loadStream(st.streamIndex, st.pos); closeSettings(); return; }
      if (d.sub != null) { if (d.sub === 'off') selectSub(null); else selectSub(Number(d.sub)); return; }
      if (d.tid) { set('sid', d.tid); set('sub-visibility', 'yes'); st.sid = -1; renderSettingsPane(); return; }
      if (d.cat) { const wantDub = d.cat === 'dub'; if (wantDub === st.item.dub) return; switchDubInPlayer(wantDub); return; }
      if (d.aid) { set('aid', d.aid); renderSettingsPane(); return; }
      if (d.alang != null) { Settings.set('audioLang', d.alang); await Settings.push(); loadStream(st.streamIndex, st.pos); closeSettings(); return; }
      if (d.srv) { const i = Number(d.srv); if (i !== st.streamIndex) { Memory.rememberSource(st.item.title, st.streams[i].source); loadStream(i, st.pos); } closeSettings(); return; }
      if (d.dl) { startDownload(st.item); closeSettings(); return; }
      if (d.dlh) { Settings.set('downloadHeight', Number(d.dlh)); renderSettingsPane(); return; }
      if (d.speed) { setSpeed(Number(d.speed)); return; }
      if (d.toggle === 'stats') { st.statsOn = !st.statsOn; renderStats(); renderSettingsPane(); return; }
      if (d.toggle) { Settings.set(d.toggle, !S[d.toggle]); if (d.toggle === 'autoplay') ui.autonext.classList.toggle('on', S.autoplay); renderSettingsPane(); }
    });
    ui.settingsWrap.addEventListener('input', e => { if (e.target.id === 'vsd-size-slider') { Settings.set('subPx', Number(e.target.value)); const v = byId('vsd-size-val'); if (v) v.textContent = `${e.target.value}px`; applySubStyle(); } });
    async function switchDubInPlayer(dub) {
      const it = st.item;
      try {
        const list = await api.get('title', { kind: 'anime', id: String(it.title.id), dub: dub ? '1' : '0' });
        if (list.dub !== dub) { notify(dub ? 'No dub for this one' : 'Only the dub is available'); return; }
        Settings.set('dub', dub);
        const cur = Item.ep(it);
        let idx = list.episodes.findIndex(e => cur && e.number === cur.number);
        if (idx < 0) idx = 0;
        closeSettings();
        start(Item.make(it.title, list.episodes, idx, dub, st.pos > 5 ? st.pos : null, it.chain));
      } catch (err) { notify(String(err)); }
    }

    function onKey(e) {
      if (!st.open) return;
      if (e.target && /INPUT|TEXTAREA|SELECT/.test(e.target.tagName)) { if (e.key === 'Escape') { e.target.blur(); } return; }
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
        case 'f': case 'F': case 'F11': toggleFullscreen(); break;
        case 'c': case 'C': if (st.subs.length) selectSub(st.sid == null ? 0 : null); break;
        case 'n': case 'N': if (Item.next(st.item)) goNext(); break;
        case 'e': case 'E': toggleDrawer(); break;
        case 'p': case 'P': toggleMini(); break;
        case 's': case 'S': toggleSettings('home'); break;
        case 'i': case 'I': st.statsOn = !st.statsOn; renderStats(); break;
        case '[': setSpeed((S.speed || 1) - 0.25); break;
        case ']': setSpeed((S.speed || 1) + 0.25); break;
        case 'Backspace': setSpeed(1); break;
        case 'Escape':
          if (root.classList.contains('settings-open')) closeSettings();
          else if (root.classList.contains('drawer-open')) closeDrawer();
          else if (st.fullscreen) toggleFullscreen(false);
          else if (st.mini) toggleMini(false);
          else close();
          break;
        case 'Home': seek(0, true); break;
        case 'End': if (st.dur) seek(st.dur - 1, true); break;
        default: if (/^[0-9]$/.test(k) && st.dur) seek(st.dur * Number(k) / 10, true); else handled = false;
      }
      if (handled) { e.preventDefault(); e.stopPropagation(); showControls(); }
    }
    let clickTimer;
    root.addEventListener('mousemove', () => { if (st.open) showControls(); });
    root.addEventListener('mouseleave', () => { if (st.open && !st.paused) scheduleHide(); });
    root.addEventListener('click', e => {
      if (e.target.closest('.topbar, .controls, .skip-btn, .next-ep-overlay, .end-actions, .settings-panel-wrap, .drawer, .m-center, .scanning-overlay, .stats, .notif-stack')) return;
      if (root.classList.contains('settings-open')) { closeSettings(); return; }
      if (root.classList.contains('drawer-open')) { closeDrawer(); return; }
      if (e.detail === 2) { clearTimeout(clickTimer); toggleFullscreen(); return; }
      clearTimeout(clickTimer);
      clickTimer = setTimeout(() => togglePlay(), 230);
    });
    root.addEventListener('wheel', e => { if (e.target.closest('.drawer, .settings-panel-wrap')) return; setVolume(S.volume + (e.deltaY < 0 ? 5 : -5)); }, { passive: true });
    document.addEventListener('click', e => { if (!e.target.closest('#season-pill-wrap')) ui.seasonDrop.classList.remove('open'); });
    ui.playpause.onclick = togglePlay; ui.mPlay.onclick = togglePlay;
    ui.rew.onclick = () => seek(-S.skipSeconds); ui.mRew.onclick = () => seek(-S.skipSeconds);
    ui.ffw.onclick = () => seek(S.skipSeconds); ui.mFwd.onclick = () => seek(S.skipSeconds);
    ui.mute.onclick = toggleMute;
    ui.volume.oninput = () => setVolume(Number(ui.volume.value) * 100, true);
    ui.back.onclick = () => close();
    ui.fullscreen.onclick = () => toggleFullscreen();
    ui.pip.onclick = () => toggleMini();
    ui.nextEpisode.onclick = goNext;
    ui.episodesToggle.onclick = toggleDrawer;
    ui.infoBtn.onclick = () => { if (st.item) { openInfo(st.item.title); } };
    ui.listBtn.onclick = () => toggleList(st.item.title).then(() => ui.listBtn.classList.toggle('on', inList(st.item.title)));
    ui.nextNow.onclick = goNext;
    ui.nextCancel.onclick = () => { st.dismissedNext = true; ui.nextOv.classList.add('hidden'); };
    ui.endReplay.onclick = () => { st.ended = false; ui.endActions.classList.add('hidden'); seek(0, true); set('pause', 'no'); };
    ui.endNext.onclick = goNext;
    $$('.settings-trigger', root).forEach(b => b.onclick = () => toggleSettings(b.dataset.openTab || 'home'));
    const scrubAt = e => { const r = ui.progress.getBoundingClientRect(); return clamp((e.clientX - r.left) / r.width, 0, 1); };
    ui.seek.addEventListener('input', () => { st.scrubbing = true; const f = Number(ui.seek.value) / 100; ui.play.style.width = `${f * 100}%`; ui.thumb.style.left = `${f * 100}%`; ui.time.textContent = `${tclock(f * st.dur)} / ${tclock(st.dur)}`; });
    ui.seek.addEventListener('change', () => { st.scrubbing = false; seek(Number(ui.seek.value) / 100 * st.dur, true); });
    ui.progress.addEventListener('mousedown', e => { if (e.target === ui.seek) return; if (!st.dur) return; seek(scrubAt(e) * st.dur, true); });
    listen('mpv', onEvent);
    setInterval(tick, 1000);
    refreshSkip();
    paintVolume();
    function media(key) {
      if (!st.open || !st.item) return;
      if (key === 'playpause') togglePlay();
      else if (key === 'next') goNext();
      else if (key === 'prev') { if (st.pos > 5) seek(0, true); else if (st.item.index > 0 && !Item.isMovie(st.item)) start(Item.at(st.item, st.item.index - 1)); }
      else if (key === 'stop') close();
    }
    function pause() { if (st.open && st.item && !st.paused) set('pause', 'yes'); }
    return { init, start, close, set, applySubStyle, refreshSkip, presence, media, pause, get open() { return st.open; } };
  })();

  $$('.tab[data-mode]').forEach(b => b.onclick = () => showMode(b.dataset.mode));
  byId('nav-search-btn').onclick = openSearch;
  byId('nav-history').onclick = () => showPage('history');
  byId('nav-downloads').onclick = () => showPage('downloads');
  byId('nav-settings').onclick = openSettings;
  byId('nav-profile').onclick = openAccount;
  $$('.winctl button').forEach(b => b.onclick = () => invoke('win', { action: b.dataset.win }));
  byId('dragbar').addEventListener('dblclick', () => invoke('win', { action: 'maximize' }));
  window.addEventListener('scroll', () => { byId('topnav').classList.toggle('scrolled', window.scrollY > 40); }, { passive: true });
  let windowFull = false;
  document.addEventListener('keydown', e => {
    if (e.key === 'F11') { e.preventDefault(); windowFull = !windowFull; invoke('win', { action: 'fullscreen', on: windowFull }); return; }
    if (Player.open) return;
    if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K')) { e.preventDefault(); openSearch(); return; }
    if (e.target && /INPUT|TEXTAREA|SELECT/.test(e.target.tagName)) { if (e.key === 'Escape') { e.target.blur(); if (ov.classList.contains('open')) closeSearch(); } return; }
    if (e.key === 'Escape') { if (!ctx.hidden) closeMenus(); else if (ov.classList.contains('open')) closeSearch(); else if (infoOv.classList.contains('open')) closeInfo(); return; }
    if (e.key === '/') { e.preventDefault(); openSearch(); }
  });

  const newer = (a, b) => { const x = String(a || '').split('.').map(Number), y = String(b || '').split('.').map(Number); for (let i = 0; i < 3; i++) { if ((x[i] || 0) > (y[i] || 0)) return true; if ((x[i] || 0) < (y[i] || 0)) return false; } return false; };
  async function checkUpdates() {
    if (!S.updates || !App.boot.version) return;
    try {
      const r = await fetch(`${App.boot.host}/desktop/version.json`, { cache: 'no-store' });
      if (!r.ok) return;
      const j = await r.json();
      if (j && newer(j.version, App.boot.version)) toast(`viel ${j.version} is out. Click to download.`, 9000, () => invoke('open_external', { url: j.url || `${App.boot.host}/desktop/viel-windows.zip` }));
    } catch (_) {}
  }
  listen('media', key => Player.media(key));
  listen('tray', p => { if (p && p.hidden) Player.pause(); });
  async function main() {
    document.documentElement.classList.toggle('fine-pointer', matchMedia('(hover:hover) and (pointer:fine)').matches);
    try { App.boot = await invoke('boot'); } catch (e) { App.boot = { host: 'https://viel.tv' }; }
    invoke('set_tray', { on: !!S.tray }).catch(() => {});
    setTimeout(checkUpdates, 4000);
    byId('foot-version').textContent = App.boot.version || '';
    setUser(App.boot.username);
    Settings.push();
    Player.init();
    invoke('downloads_active').then(rows => { rows.forEach(r => { App.active[r.key] = r; }); updateDlDot(); }).catch(() => {});
    showMode('home');
    if (!App.boot.token) { openAccount(); return; }
    api.get('me').then(r => { if (r && r.username) setUser(r.username); else if (r && r.username === null) { setUser(null); openAccount(); } }).catch(() => {});
  }
  main();
})();
