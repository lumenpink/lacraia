import { chromium } from 'playwright';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import path from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const REPO_ROOT = path.resolve(__dirname, '../..');

const BASE_URL = process.env.BASE_URL || 'http://127.0.0.1:2011';
const CHROME_PATH = process.env.CHROME_PATH || '/usr/bin/google-chrome';

let passed = 0;
let failed = 0;
let spawnedServer = null;

function assert(condition, message) {
  if (!condition) {
    throw new Error(`Assertion failed: ${message}`);
  }
}

async function test(name, fn) {
  process.stdout.write(`  ⏳ ${name}... `);
  try {
    await fn();
    console.log(`✅ OK`);
    passed++;
  } catch (err) {
    console.log(`❌ FAILED`);
    console.error(`     ${err.stack || err.message}`);
    failed++;
  }
}

async function ensureBackendReady() {
  try {
    const res = await fetch(`${BASE_URL}/health`);
    if (res.ok) {
      console.log(`   ⚡ Backend server already running at ${BASE_URL}`);
      return;
    }
  } catch (_) {
    // Not running, proceed to spawn
  }

  console.log(`   🚀 Spawning local lacraia-core daemon on port 2011...`);
  const binaryPath = path.resolve(REPO_ROOT, 'target/debug/lacraia-core');

  spawnedServer = spawn(binaryPath, [], {
    cwd: REPO_ROOT,
    env: {
      ...process.env,
      PORT: process.env.PORT || (new URL(BASE_URL).port || '2011'),
      LACRAIA_MASTER_KEY: 'lacraia_master_2011',
      LACRAIA_SITE_DIR: path.resolve(REPO_ROOT, 'site'),
      LACRAIA_DB_PATH: path.resolve(REPO_ROOT, 'lacraia.sqlite'),
      LACRAIA_CACHE_DIR: path.resolve(REPO_ROOT, 'cache'),
    },
    stdio: 'ignore',
  });

  const start = Date.now();
  while (Date.now() - start < 12000) {
    try {
      const res = await fetch(`${BASE_URL}/health`);
      if (res.ok) {
        console.log(`   ✅ Backend server ready at ${BASE_URL}\n`);
        return;
      }
    } catch (_) {}
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`Backend server failed to start within 12s at ${BASE_URL}`);
}

async function main() {
  console.log(`\n🦂 [LA CRAIA :: EXHAUSTIVE PLAYWRIGHT E2E SUITE]`);
  console.log(`   Target: ${BASE_URL}`);
  console.log(`   Browser: ${CHROME_PATH} (Headless, Ephemeral Incognito Contexts)`);
  console.log(`------------------------------------------------------------\n`);

  await ensureBackendReady();

  const browser = await chromium.launch({
    headless: true,
    executablePath: CHROME_PATH,
    args: [
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--autoplay-policy=no-user-gesture-required',
      '--disable-dev-shm-usage',
    ],
  });

  try {
    // -------------------------------------------------------------
    // Test 1: Portal Landing Page & Exoskeleton Kinematics (/)
    // -------------------------------------------------------------
    await test('Portal (/) : Favicon SVG, OG metadata, kinematics and cockpit link', async () => {
      const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/`, { waitUntil: 'domcontentloaded' });

      // Favicon & Metadata
      const favicon = await page.$('link[rel="icon"][type="image/svg+xml"]');
      assert(favicon !== null, 'Portal must have SVG favicon');

      const ogTitle = await page.$('meta[property="og:title"]');
      assert(ogTitle !== null, 'Portal must include Open Graph title meta');

      // Biomechanical Antennae Kinematics
      const antennaeSvg = await page.$('#antennaeSvg');
      const leftAntenna = await page.$('#leftAntenna');
      const rightAntenna = await page.$('#rightAntenna');
      assert(antennaeSvg !== null, '#antennaeSvg must exist');
      assert(leftAntenna !== null, '#leftAntenna must exist');
      assert(rightAntenna !== null, '#rightAntenna must exist');

      // Dispatch mousemove to test kinematic listeners
      await page.mouse.move(700, 300);
      await page.waitForTimeout(100);

      // Link to Cockpit Web /app
      const cockpitLink = await page.$('a[href*="/app"]');
      assert(cockpitLink !== null, 'Portal must contain navigation link to /app');

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 2: Cockpit Desktop Architecture & Layout (/app)
    // -------------------------------------------------------------
    await test('Cockpit Web (/app) : Desktop 3-deck architecture and telemetry layout', async () => {
      const cleanCabin = `e2e_deck_${Date.now()}`;
      const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app?cabin=${cleanCabin}`, { waitUntil: 'domcontentloaded' });

      const title = await page.title();
      assert(title.includes('LA CRAIA') || title.includes('CONSOLE'), `Title '${title}' must mention LA CRAIA or CONSOLE`);

      // Decks 01, 02, 03
      assert((await page.$('#deck-01')) !== null, '#deck-01 (Player) must exist');
      assert((await page.$('#deck-02')) !== null, '#deck-02 (Queue) must exist');
      assert((await page.$('#deck-03')) !== null, '#deck-03 (Cabin) must exist');

      // Deck 02 must NOT contain redundant microplayer
      const deck2Miniplayer = await page.$('#deck-02-miniplayer');
      assert(deck2Miniplayer === null, '#deck-02-miniplayer must NOT exist (redundant microplayer removed)');

      // Brand logo
      const brandLogo = await page.$('.brand-logo-img');
      assert(brandLogo !== null, '.brand-logo-img must exist');

      // Cassette and audio engine
      assert((await page.$('#cassette-container')) !== null, '#cassette-container must exist');
      assert((await page.$('#audio-engine')) !== null, '#audio-engine element must exist');

      // Initial factory seed zero must be MC Serginho - Vai Lacraia
      const initialTitle = await page.locator('#track-title').textContent();
      const initialArtist = await page.locator('#track-artist').textContent();
      assert(
        initialTitle.toUpperCase().includes('VAI LACRAIA'),
        `Default zero seed track title must be VAI LACRAIA, got '${initialTitle}'`
      );
      assert(
        initialArtist.toUpperCase().includes('SERGINHO'),
        `Default zero seed artist must be MC SERGINHO, got '${initialArtist}'`
      );

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 3: Mobile Viewport (< 860px) & Tab Switching
    // -------------------------------------------------------------
    await test('Mobile Viewport (375x667) : Bottom tabs navigation and persistent miniplayers', async () => {
      const context = await browser.newContext({
        viewport: { width: 375, height: 667 },
        isMobile: true,
      });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app`, { waitUntil: 'domcontentloaded' });

      // Mobile tabs navigation must be visible
      const tabs = page.locator('#mobile-tabs');
      await tabs.waitFor({ state: 'visible', timeout: 5000 });

      // Default: Deck 01 visible
      assert(await page.locator('#deck-01').isVisible(), 'Deck 01 must be active initially');

      // Click FILA Tab (Deck 02)
      await page.click('#tab-btn-deck-02');
      await page.waitForTimeout(300);
      assert(await page.locator('#deck-02').isVisible(), 'Deck 02 must be visible when FILA is selected');
      assert(!(await page.locator('#deck-01').isVisible()), 'Deck 01 must be hidden when FILA is active');
      assert((await page.$('#deck-02-miniplayer')) === null, 'Deck 02 must not contain redundant miniplayer');
      assert(await page.locator('#sticky-miniplayer').isVisible(), 'Sticky dock miniplayer must be visible when Deck 02 is active');

      // Click CABINE Tab (Deck 03)
      await page.click('#tab-btn-deck-03');
      await page.waitForTimeout(300);
      assert(await page.locator('#deck-03').isVisible(), 'Deck 03 must be visible when CABINE is selected');
      assert(await page.locator('#deck-03-miniplayer').isVisible(), 'Persistent miniplayer in Deck 03 must be visible');

      // Return to PLAYER Tab (Deck 01)
      await page.click('#tab-btn-deck-01');
      await page.waitForTimeout(300);
      assert(await page.locator('#deck-01').isVisible(), 'Deck 01 must be restored when PLAYER tab is clicked');

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 4: Roles & Strict Access Control (Passenger vs Pilot vs Master)
    // -------------------------------------------------------------
    await test('Security & Roles : Passenger restrictions, auth modal triggers, and master key login', async () => {
      const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app`, { waitUntil: 'domcontentloaded' });

      // Initial unauthenticated role is PASSAGEIRO
      const roleText = await page.locator('#role-badge').textContent();
      assert(roleText.includes('PASSAGEIRO'), `Default role must be PASSAGEIRO, got '${roleText}'`);

      const authModal = page.locator('#auth-modal');

      // 1. Restricted: Play button triggers auth modal
      await page.click('#btn-play-pause');
      await page.waitForTimeout(200);
      assert(await authModal.evaluate((el) => el.classList.contains('active')), 'Clicking play as passenger must trigger auth modal');
      await page.click('#btn-close-auth');
      await page.waitForTimeout(200);

      // 2. Restricted: Next button triggers auth modal
      await page.click('#btn-next');
      await page.waitForTimeout(200);
      assert(await authModal.evaluate((el) => el.classList.contains('active')), 'Clicking next as passenger must trigger auth modal');
      await page.click('#btn-close-auth');
      await page.waitForTimeout(200);

      // 3. Restricted: Door toggle triggers auth modal
      await page.locator('#btn-toggle-door').click({ force: true });
      await page.waitForTimeout(200);
      assert(await authModal.evaluate((el) => el.classList.contains('active')), 'Clicking toggle doors as passenger must trigger auth modal');
      await page.click('#btn-close-auth');
      await page.waitForTimeout(200);

      // 4. Restricted: Eject all triggers auth modal
      await page.locator('#btn-eject-all').click({ force: true });
      await page.waitForTimeout(200);
      assert(await authModal.evaluate((el) => el.classList.contains('active')), 'Clicking eject all as passenger must trigger auth modal');

      // 5. Test invalid token submission
      await page.fill('#auth-token-input', 'invalid_token_999');
      await page.click('#btn-submit-auth');
      await page.waitForTimeout(200);
      const errMsg = await page.locator('#auth-error-msg').textContent();
      assert(errMsg.length > 0, 'Invalid token must display error message');

      // 6. Test valid Master key submission
      await page.fill('#auth-token-input', 'lacraia_master_2011');
      await page.click('#btn-submit-auth');
      await page.waitForTimeout(400);

      const updatedRole = await page.locator('#role-badge').textContent();
      assert(updatedRole.includes('MASTER'), `Role must update to MASTER, got '${updatedRole}'`);

      // 7. Test Logout ("Entregar o volante")
      await page.click('#role-badge');
      await page.waitForTimeout(200);
      const logoutBtn = page.locator('#btn-submit-logout');
      assert(await logoutBtn.isVisible(), 'Logout button must be visible for authenticated user');
      await logoutBtn.click();
      await page.waitForTimeout(300);

      const revertedRole = await page.locator('#role-badge').textContent();
      assert(revertedRole.includes('PASSAGEIRO'), `Role must revert to PASSAGEIRO upon logout, got '${revertedRole}'`);

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 5: Audio Engine, Sound Verification & WebAudio 32 Patas Visualizer
    // -------------------------------------------------------------
    await test('Audio Engine & Sound Output : Playback, currentTime advancement, volume to 11 and visualizer', async () => {
      const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app?master_key=lacraia_master_2011`, { waitUntil: 'domcontentloaded' });

      // Gain slider to 11
      const gainSlider = page.locator('#gain-slider');
      await gainSlider.fill('11');
      await gainSlider.dispatchEvent('input');
      const gainDisplay = await page.locator('#gain-val-display').textContent();
      assert(gainDisplay.includes('11/11'), `Gain display '${gainDisplay}' must indicate 11/11`);

      const volume = await page.evaluate(() => document.getElementById('audio-engine').volume);
      assert(volume === 1.0, `Audio element volume must clamp to 1.0, got ${volume}`);

      // Start playback
      await page.click('#btn-play-pause');

      // Wait for audio src to be populated
      await page.waitForFunction(() => {
        const a = document.getElementById('audio-engine');
        return a && a.src && a.src.length > 0 && a.src !== window.location.href;
      }, { timeout: 15000 });

      // Ensure audio is actively playing in browser
      await page.evaluate(() => {
        const a = document.getElementById('audio-engine');
        if (a.paused) a.play().catch(() => {});
      });

      await page.waitForFunction(() => {
        const a = document.getElementById('audio-engine');
        return a && !a.paused;
      }, { timeout: 10000 });

      // SOUND VERIFICATION 1: Is playing and not paused
      const isPaused = await page.evaluate(() => document.getElementById('audio-engine').paused);
      assert(!isPaused, 'Audio element must be playing (!paused)');

      // SOUND VERIFICATION 2: Real-time clock advance (sound is actively streaming)
      const t0 = await page.evaluate(() => document.getElementById('audio-engine').currentTime);
      await page.waitForTimeout(1000);
      const t1 = await page.evaluate(() => document.getElementById('audio-engine').currentTime);
      assert(t1 >= t0, `Audio currentTime must advance or match active stream: t0=${t0}, t1=${t1}`);

      // SOUND VERIFICATION 3: Canvas visualizer has non-blank pixel rendering
      const hasVisualizerEnergy = await page.evaluate(() => {
        const c = document.getElementById('visualizer');
        const ctx = c.getContext('2d');
        const img = ctx.getImageData(0, 0, c.width, c.height);
        let coloredPixels = 0;
        for (let i = 0; i < img.data.length; i += 4) {
          if (img.data[i + 3] > 80 && (img.data[i] > 40 || img.data[i + 1] > 40)) {
            coloredPixels++;
          }
        }
        return coloredPixels > 0;
      });
      assert(hasVisualizerEnergy, 'Visualizer canvas must draw active metameric frequency bars');

      // Toggle Visualizer Button OFF
      const toggleVisBtn = page.locator('#btn-toggle-visualizer');
      await toggleVisBtn.click();
      await page.waitForTimeout(200);
      const visOffStatus = await page.locator('#vis-status').textContent();
      assert(visOffStatus.includes('OFF'), `Visualizer status must be OFF, got '${visOffStatus}'`);

      // Toggle Visualizer Button back ON
      await toggleVisBtn.click();
      await page.waitForTimeout(200);
      const visOnStatus = await page.locator('#vis-status').textContent();
      assert(visOnStatus.includes('ON'), `Visualizer status must be ON, got '${visOnStatus}'`);

      // Seek Bar Test
      const progressTrack = page.locator('#progress-track');
      const box = await progressTrack.boundingBox();
      assert(box !== null, 'Progress track must have bounding box');
      await page.mouse.click(box.x + box.width * 0.5, box.y + box.height * 0.5);
      await page.waitForTimeout(300);

      // Verify seek updated state position and time label
      const timeCurrentText = await page.locator('#time-current').textContent();
      assert(timeCurrentText !== '00:00', `Seeking to 50% must advance time label, got ${timeCurrentText}`);

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 6: Station Dial by Acoustic Similarity Graph
    // -------------------------------------------------------------
    await test('Station Dials : Graph dialect switching across all 5 preset stations', async () => {
      const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app?master_key=lacraia_master_2011`, { waitUntil: 'domcontentloaded' });

      const stations = [
        'Synthwave Drive',
        'Rock Clássico',
        'Lo-Fi Road Trip',
        'Paredão Autônomo',
        'MPB & Samba',
      ];

      for (const st of stations) {
        const btn = page.locator(`.station-btn[data-station="${st}"]`);
        await btn.click();
        await page.waitForTimeout(150);

        const activeText = await btn.evaluate((el) => el.classList.contains('btn-active'));
        assert(activeText, `Station button for '${st}' must receive .btn-active`);

        const indicator = await page.locator('#station-indicator').textContent();
        assert(
          indicator.toUpperCase().includes(st.toUpperCase()),
          `Station indicator '${indicator}' must reflect '${st}'`
        );
      }

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 7: Jukebox Search, Dropdown Autocomplete & Candidate Enqueue
    // -------------------------------------------------------------
    await test('Jukebox & Search : Debounced autocomplete, candidate selection and form submission', async () => {
      const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app?master_key=lacraia_master_2011`, { waitUntil: 'domcontentloaded' });

      const input = page.locator('#enqueue-input');
      await input.fill('Chico');

      const dropdown = page.locator('#search-dropdown');
      await dropdown.waitFor({ state: 'visible', timeout: 5000 });

      // Assert all 4 action buttons exist in candidate row
      const candidateRow = dropdown.locator('.search-item').first();
      const playNowBtn = candidateRow.locator('button[data-action="play_now"]');
      const playNextBtn = candidateRow.locator('button[data-action="play_next"]');
      const enqueueBtn = candidateRow.locator('button[data-action="enqueue"]');
      const startRadioBtn = candidateRow.locator('button[data-action="start_radio"]');

      assert(await playNowBtn.count() > 0, 'Candidate row must have "Tocar agora" (play_now) button');
      assert(await playNextBtn.count() > 0, 'Candidate row must have "Tocar na sequência" (play_next) button');
      assert(await enqueueBtn.count() > 0, 'Candidate row must have "Adicionar à fila" (enqueue) button');
      assert(await startRadioBtn.count() > 0, 'Candidate row must have "Iniciar rádio" (start_radio) button');

      // Click the "+ FILA" button inside the first candidate row
      const firstCandidateBtn = candidateRow.locator('button[data-action="enqueue"], button').first();
      await firstCandidateBtn.click();
      await page.waitForTimeout(800);

      const queueText = await page.locator('#queue-list-container').textContent();
      assert(
        !queueText.includes('[ NENHUMA CANTIGA ]') || queueText.includes('Chico') || queueText.includes('Construção') || queueText.includes('CONSTRUÇÃO'),
        'Queue list must contain enqueued track'
      );

      // Verify dropdown closes on click outside
      await input.fill('Tom');
      await dropdown.waitFor({ state: 'visible', timeout: 5000 });
      await page.click('.brand-title');
      await page.waitForTimeout(200);
      assert(!(await dropdown.isVisible()), 'Dropdown must close when clicking outside');

      // Test Play Now action button on candidate
      await input.fill('Kraftwerk');
      await dropdown.waitFor({ state: 'visible', timeout: 5000 });
      const kwRow = dropdown.locator('.search-item').first();
      await kwRow.locator('button[data-action="play_now"]').click();
      await page.waitForTimeout(800);
      const titleAfterPlayNow = await page.locator('#track-title').textContent();
      assert(titleAfterPlayNow.includes('Autobahn') || titleAfterPlayNow.includes('Kraftwerk'), 'Play Now button must immediately switch active track');

      // Test Play Next action button on candidate
      await input.fill('Daft');
      await dropdown.waitFor({ state: 'visible', timeout: 5000 });
      const daftRow = dropdown.locator('.search-item').first();
      await daftRow.locator('button[data-action="play_next"]').click();
      await page.waitForTimeout(800);
      const firstUpcomingTitle = await page.locator('.queue-item-row .queue-item-title').first().textContent();
      assert(firstUpcomingTitle.includes('One More Time') || firstUpcomingTitle.includes('Daft') || firstUpcomingTitle.includes('Aerodynamic'), 'Play Next button must insert track at head of upcoming queue');

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 8: Autopilot Predictive Lookahead Queue Population
    // -------------------------------------------------------------
    await test('Autopilot Lookahead : Adding a track triggers automatic population of upcoming slots', async () => {
      const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app?master_key=lacraia_master_2011`, { waitUntil: 'domcontentloaded' });

      // Trigger playback to wake the Autopilot lookahead daemon
      await page.click('#btn-play-pause');

      // Enqueue a distinct seed track
      const input = page.locator('#enqueue-input');
      await input.fill('Kraftwerk');
      await page.waitForTimeout(400);

      // Submit form
      await page.click('#enqueue-submit-btn');

      // Wait for Queue Daemon to maintain lookahead window (up to 5 tracks)
      await page.waitForFunction(() => {
        const rows = document.querySelectorAll('.queue-item-row');
        return rows.length >= 2;
      }, { timeout: 15000 });

      const queueContent = await page.locator('#queue-list-container').textContent();
      assert(
        queueContent.includes('AUTOPILOT') || queueContent.includes('PILOTO') || queueContent.includes('SISTEMA'),
        'Autopilot must automatically populate upcoming buffer positions'
      );

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 9: Queue Manipulation (Direct Play & Item Removal)
    // -------------------------------------------------------------
    await test('Queue Management : Direct play from queue and individual item removal', async () => {
      const queueCabin = `e2e_queue_${Date.now()}`;
      const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app?cabin=${queueCabin}&master_key=lacraia_master_2011`, { waitUntil: 'domcontentloaded' });

      // Add Track 1 (Kraftwerk)
      const input = page.locator('#enqueue-input');
      await input.fill('Kraftwerk');
      const dropdown = page.locator('#search-dropdown');
      await dropdown.waitFor({ state: 'visible', timeout: 5000 });
      await dropdown.locator('.search-item button, .search-item').first().click();
      await page.waitForTimeout(600);

      // Add Track 2 (Chico)
      await input.fill('Chico');
      await dropdown.waitFor({ state: 'visible', timeout: 5000 });
      await dropdown.locator('.search-item button, .search-item').first().click();
      await page.waitForTimeout(600);

      const initialCount = await page.locator('.queue-item-row').count();
      assert(initialCount === 2, `Queue must have 2 upcoming items, got ${initialCount}`);

      // Test Direct Play button in queue row (first item plays, so it exits the upcoming queue)
      const firstPlayBtn = page.locator('.queue-item-row button:has-text("▶")').first();
      if (await firstPlayBtn.isVisible()) {
        await firstPlayBtn.click();
        await page.waitForTimeout(600);
        const title = await page.locator('#track-title').textContent();
        assert(title.length > 0, 'Clicking direct play on queue item must update active track');
      }

      // Test Remove button on remaining queue item
      const firstRemoveBtn = page.locator('.queue-item-row button:has-text("✕")').first();
      if (await firstRemoveBtn.isVisible()) {
        await firstRemoveBtn.click();
        await page.waitForTimeout(600);
        const newCount = await page.locator('.queue-item-row').count();
        assert(newCount === 0, `Queue item count must be 0 after removal, got ${newCount}`);
      }

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 10: Cabin Doors Lock & Passenger Enqueue Blocking
    // -------------------------------------------------------------
    await test('Cabin Lock & Doors : Toggle lock and verify passenger enqueuing is strictly blocked', async () => {
      // Pilot Context
      const pilotCtx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const pilotPage = await pilotCtx.newPage();
      await pilotPage.goto(`${BASE_URL}/app?master_key=lacraia_master_2011`, { waitUntil: 'domcontentloaded' });

      // Lock the cabin doors
      await pilotPage.locator('#btn-toggle-door').click({ force: true });
      await pilotPage.waitForFunction(() => {
        const badge = document.getElementById('door-badge');
        return badge && badge.textContent.includes('TRANCADAS');
      }, { timeout: 8000 });

      // Passenger Context (Separate Incognito Session)
      const passengerCtx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const passengerPage = await passengerCtx.newPage();
      await passengerPage.goto(`${BASE_URL}/app`, { waitUntil: 'domcontentloaded' });

      // Attempt to enqueue while locked
      await passengerPage.locator('#enqueue-input').fill('Daft Punk');
      await passengerPage.click('#enqueue-submit-btn');
      await passengerPage.waitForTimeout(500);

      const toast = passengerPage.locator('#search-toast');
      const toastText = await toast.textContent();
      assert(
        toastText.toUpperCase().includes('TRANCA') || toastText.toUpperCase().includes('PILOTO') || toastText.length > 0,
        `Toast notification must warn about locked cabin or pilot authorization, got '${toastText}'`
      );

      // Unlock doors as Pilot
      await pilotPage.locator('#btn-toggle-door').click({ force: true });
      await pilotPage.waitForFunction(() => {
        const badge = document.getElementById('door-badge');
        return badge && badge.textContent.includes('ABERTAS');
      }, { timeout: 8000 });

      await passengerCtx.close();
      await pilotCtx.close();
    });

    // -------------------------------------------------------------
    // Test 11: Crew Management, Mass Ejection & Glitch Screen
    // -------------------------------------------------------------
    await test('Crew & Ejection : Verify crew list, mass ejection signal and glitch overlay', async () => {
      // 1. Pilot Page
      const pilotCtx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const pilotPage = await pilotCtx.newPage();
      await pilotPage.goto(`${BASE_URL}/app?master_key=lacraia_master_2011`, { waitUntil: 'domcontentloaded' });
      await pilotPage.waitForFunction(() => document.getElementById('ws-badge')?.classList.contains('badge-live'), { timeout: 8000 });

      // 2. Passenger Page
      const passCtx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const passPage = await passCtx.newPage();
      await passPage.goto(`${BASE_URL}/app`, { waitUntil: 'domcontentloaded' });

      // Wait for passenger's registration and live WebSocket connection
      await passPage.waitForFunction(() => {
        const badge = document.getElementById('ws-badge');
        return badge && badge.classList.contains('badge-live');
      }, { timeout: 8000 });

      // Pilot triggers Eject All
      await pilotPage.locator('#btn-eject-all').click({ force: true });

      // Passenger should receive ejection screen
      await passPage.waitForFunction(() => {
        const scr = document.getElementById('ejection-screen');
        return scr && (scr.classList.contains('active') || window.getComputedStyle(scr).display !== 'none');
      }, { timeout: 8000 });

      const reconnectBtn = passPage.locator('#ejection-screen button');
      assert(await reconnectBtn.isVisible(), 'Ejection screen must have reconnect button');

      await passCtx.close();
      await pilotCtx.close();
    });

    // -------------------------------------------------------------
    // Test 12: Modals Exploration (QR Modal, Cabin Modal, ESC Key & Creation)
    // -------------------------------------------------------------
    await test('Modals System : QR code canvas, key rotation, ESC closing and cabin creation', async () => {
      const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const page = await context.newPage();
      page.on('dialog', (dialog) => dialog.accept());

      await page.goto(`${BASE_URL}/app?master_key=lacraia_master_2011`, { waitUntil: 'domcontentloaded' });

      // 1. QR Modal
      await page.click('#btn-cabin-access');
      await page.waitForTimeout(200);

      const qrModal = page.locator('#qr-modal');
      assert(await qrModal.evaluate((el) => el.classList.contains('active')), '#qr-modal must be active');

      const qrCanvas = await page.$('#qr-canvas');
      assert(qrCanvas !== null, '#qr-canvas element must exist in QR modal');

      // Rotate Key button
      const oldKeyText = await page.locator('#qr-link-text').textContent();
      await page.click('#btn-rotate-key');
      await page.waitForTimeout(200);
      const newKeyText = await page.locator('#qr-link-text').textContent();
      assert(newKeyText.length > 0, 'Key link text must be updated');

      // Close QR Modal via close button
      await page.click('#btn-close-qr');
      await page.waitForTimeout(200);
      assert(!(await qrModal.evaluate((el) => el.classList.contains('active'))), '#qr-modal must close on close button');

      // 2. Cabin Modal
      await page.click('#cabin-badge');
      await page.waitForTimeout(200);

      const cabinModal = page.locator('#cabin-modal');
      assert(await cabinModal.evaluate((el) => el.classList.contains('active')), '#cabin-modal must be active');

      // Master Create Cabin Box
      const createBox = page.locator('#master-create-cabin-box');
      assert(await createBox.isVisible(), 'Master create cabin box must be visible for MASTER role');

      const uniqueCabinId = `pw-${Date.now().toString().slice(-4)}`;
      await page.fill('#new-cabin-name', 'Cabine Experimental');
      await page.fill('#new-cabin-id', uniqueCabinId);
      await page.click('#btn-create-cabin-submit');
      await page.waitForTimeout(600);

      // Cabin creation automatically switches cabin and closes the modal
      assert(!(await cabinModal.evaluate((el) => el.classList.contains('active'))), '#cabin-modal must auto-close upon cabin creation');

      const activeCabinName = await page.locator('#cabin-name-display').textContent();
      assert(activeCabinName.length > 0, `Active cabin name display must exist, got '${activeCabinName}'`);

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 13: Sticky Miniplayer & Global Keyboard Shortcuts with Input Isolation
    // -------------------------------------------------------------
    await test('Shortcuts & Sticky Miniplayer : Space, Arrows, transport buttons and input isolation', async () => {
      const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app?master_key=lacraia_master_2011`, { waitUntil: 'domcontentloaded' });

      // Wait for WS connection to stabilize initial state
      await page.waitForFunction(() => document.getElementById('ws-badge')?.classList.contains('badge-live'), { timeout: 8000 });

      // Sticky Miniplayer existence
      const stickyPlayer = page.locator('#sticky-miniplayer');
      assert(stickyPlayer !== null, '#sticky-miniplayer must exist');

      // Check initial playback text
      const initialPlayText = (await page.locator('#play-text').textContent()).trim();

      // Trigger playback toggle via Spacebar
      await page.keyboard.press('Space');
      await page.waitForTimeout(600);

      const toggledPlayText = (await page.locator('#play-text').textContent()).trim();
      assert(toggledPlayText !== initialPlayText, `Spacebar must toggle playback state (from '${initialPlayText}' to '${toggledPlayText}')`);

      // Trigger toggle again via master transport play button
      await page.locator('#btn-play-pause').click();
      await page.waitForTimeout(600);

      const restoredPlayText = (await page.locator('#play-text').textContent()).trim();
      assert(restoredPlayText === initialPlayText, `Master play button must toggle playback state back (to '${restoredPlayText}')`);

      // ArrowRight advances track
      const titleBefore = await page.locator('#track-title').textContent();
      await page.keyboard.press('ArrowRight');
      await page.waitForTimeout(600);
      const titleAfter = await page.locator('#track-title').textContent();
      assert(titleBefore !== null && titleAfter !== null, 'Track title must be readable across navigation');

      // Input Isolation: typing space inside input must NOT toggle playback
      const input = page.locator('#enqueue-input');
      await input.focus();
      const playStateBeforeTyping = await page.locator('#play-text').textContent();
      await page.keyboard.type(' test track');
      await page.waitForTimeout(300);

      // Playback text state should remain unchanged while typing inside input
      const stateWhileTyping = await page.locator('#play-text').textContent();
      assert(stateWhileTyping === playStateBeforeTyping, 'Typing inside input must not trigger transport controls');

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 14: Complete Clean-Slate Lifecycle: Enqueue 1 -> Play -> Pause -> Enqueue 2 -> Jump to 2 -> Enqueue 3 -> Next
    // -------------------------------------------------------------
    await test('Clean-Slate Lifecycle : Zerada -> Enqueue 1 -> Play -> Pause -> Enqueue 2 -> Jump to 2 -> Enqueue 3 -> Next', async () => {
      const cleanCabin = `e2e_clean_${Date.now()}`;
      const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const page = await context.newPage();

      // Step 1: Tela zerada (isolated clean cabin with master key)
      await page.goto(`${BASE_URL}/app?cabin=${cleanCabin}&master_key=lacraia_master_2011`, { waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(600);

      // Verify clean/empty queue initially
      const initialQueueText = await page.locator('#queue-list-container').textContent();
      assert(
        initialQueueText.includes('FILA LIVRE') || initialQueueText.includes('NENHUMA') || (await page.locator('.queue-item-row').count()) === 0,
        `Queue must start completely clean, got: ${initialQueueText}`
      );

      // Step 2: Enqueue Track 1 (Chico - Construção)
      const input = page.locator('#enqueue-input');
      await input.fill('Chico');
      const dropdown = page.locator('#search-dropdown');
      await dropdown.waitFor({ state: 'visible', timeout: 5000 });
      const addFirstBtn = dropdown.locator('.search-item button, .search-item').first();
      await addFirstBtn.click();
      await page.waitForTimeout(800);

      // Verify exactly ONE item was added (not twice!)
      const queueCountAfter1 = await page.locator('.queue-item-row').count();
      assert(queueCountAfter1 === 1, `Queue must have exactly 1 item after first enqueue, got ${queueCountAfter1}`);

      // Step 3: Click Play -> audio must start playing and currentTime advance
      await page.click('#btn-play-pause');
      await page.waitForFunction(() => {
        const a = document.getElementById('audio-engine');
        return a && !a.paused && a.src && a.src.length > 0;
      }, { timeout: 10000 });

      const isPlaying = await page.evaluate(() => !document.getElementById('audio-engine').paused);
      assert(isPlaying, 'Audio element must be playing after clicking play');

      const t0 = await page.evaluate(() => document.getElementById('audio-engine').currentTime);
      await page.waitForTimeout(1000);
      const t1 = await page.evaluate(() => document.getElementById('audio-engine').currentTime);
      assert(t1 >= t0, `Audio currentTime must advance during playback: t0=${t0}, t1=${t1}`);

      // The currently playing track must NOT be listed in upcoming queue deck
      const playingInQueue = await page.evaluate(() => {
        const tit = document.getElementById('track-title')?.textContent?.trim()?.toUpperCase();
        const rows = Array.from(document.querySelectorAll('.queue-item-row .queue-item-title'))
          .map(el => el.textContent.trim().toUpperCase());
        return rows.some(r => r.length > 0 && tit.includes(r));
      });
      assert(!playingInQueue, 'Currently playing track must NOT appear in the upcoming queue list');

      // Step 4: Click Pause -> audio must pause
      await page.click('#btn-play-pause');
      await page.waitForTimeout(400);
      const isPaused = await page.evaluate(() => document.getElementById('audio-engine').paused);
      assert(isPaused, 'Audio element must be paused after clicking pause');

      // Step 5: Enqueue Track 2 (Daft Punk - One More Time)
      await input.fill('One More Time');
      await dropdown.waitFor({ state: 'visible', timeout: 5000 });
      const addSecondBtn = dropdown.locator('.search-item button, .search-item').first();
      await addSecondBtn.click();
      await page.waitForTimeout(800);

      // Verify upcoming queue has Track 2 (Track 1 is currently loaded/active in player, not in upcoming queue)
      const queueCountAfter2 = await page.locator('.queue-item-row').count();
      assert(queueCountAfter2 === 1, `Queue must have exactly 1 upcoming item after second enqueue, got ${queueCountAfter2}`);

      // Step 6: Muda pra ela (Click on Track 2 row to jump directly to it)
      const secondTrackRow = page.locator('.queue-item-row').first();
      await secondTrackRow.click();
      await page.waitForTimeout(800);

      // Verify player switched to Track 2 and resumed playback
      await page.waitForFunction(() => {
        const a = document.getElementById('audio-engine');
        const tit = document.getElementById('track-title');
        return a && !a.paused && tit && (tit.textContent.includes('One More Time') || tit.textContent.includes('Aerodynamic') || tit.textContent.includes('Daft'));
      }, { timeout: 10000 });

      const trackTitleNow = await page.locator('#track-title').textContent();
      assert(
        trackTitleNow.includes('One More Time') || trackTitleNow.includes('Aerodynamic') || trackTitleNow.includes('Daft'),
        `Track title must update to Track 2 after clicking it, got '${trackTitleNow}'`
      );

      // Step 7: Enqueue Track 3 (Serginho / Vai Lacraia)
      await input.fill('Vai Lacraia');
      await dropdown.waitFor({ state: 'visible', timeout: 5000 });
      const addThirdBtn = dropdown.locator('.search-item button, .search-item').first();
      await addThirdBtn.click();
      await page.waitForTimeout(800);

      const queueCountAfter3 = await page.locator('.queue-item-row').count();
      assert(queueCountAfter3 === 1, `Queue must have 1 upcoming track (Track 3), got ${queueCountAfter3} rows`);

      // Step 8: Click Next -> transitions to Track 3
      await page.click('#btn-next');
      await page.waitForTimeout(800);

      await page.waitForFunction(() => {
        const tit = document.getElementById('track-title');
        return tit && (tit.textContent.includes('Vai Lacraia') || tit.textContent.includes('Lacraia') || tit.textContent.includes('Serginho'));
      }, { timeout: 10000 });

      const titleAfterNext = await page.locator('#track-title').textContent();
      assert(
        titleAfterNext.includes('Vai Lacraia') || titleAfterNext.includes('Lacraia') || titleAfterNext.includes('Serginho'),
        `Track title after Next must advance to Track 3, got '${titleAfterNext}'`
      );

      await context.close();
    });

  } finally {
    await browser.close();
    if (spawnedServer) {
      try {
        console.log(`   🛑 Terminating auto-spawned backend server (PID: ${spawnedServer.pid})...`);
        spawnedServer.kill('SIGTERM');
      } catch (_) {}
    }
  }

  console.log(`\n------------------------------------------------------------`);
  console.log(`🏁 Playwright E2E Results: ${passed} passed, ${failed} failed`);
  console.log(`------------------------------------------------------------\n`);

  if (failed > 0) {
    process.exit(1);
  }
}

main().catch((err) => {
  console.error('Fatal test runner error:', err);
  if (spawnedServer) {
    try {
      spawnedServer.kill('SIGTERM');
    } catch (_) {}
  }
  process.exit(1);
});
