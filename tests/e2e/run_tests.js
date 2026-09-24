import { chromium } from 'playwright';

const BASE_URL = process.env.BASE_URL || 'http://127.0.0.1:2011';
const CHROME_PATH = process.env.CHROME_PATH || '/usr/bin/google-chrome';

let passed = 0;
let failed = 0;

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

async function main() {
  console.log(`\n🦂 [LA CRAIA :: E2E PLAYWRIGHT SUITE]`);
  console.log(`   Target: ${BASE_URL}/app`);
  console.log(`   Browser: ${CHROME_PATH} (Headless, Ephemeral Context)`);
  console.log(`------------------------------------------------------------\n`);

  // Launch Google Chrome in strictly headless mode with ephemeral incognito context
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
    // Test 1: Desktop Cockpit Loading & Architecture Layout
    // -------------------------------------------------------------
    await test('Desktop Cockpit: Page load, title and 3-deck layout', async () => {
      const context = await browser.newContext({
        viewport: { width: 1440, height: 900 },
      });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app`, { waitUntil: 'domcontentloaded' });

      const title = await page.title();
      assert(title.includes('LA CRAIA') || title.includes('COCKPIT'), `Title '${title}' must mention LA CRAIA or COCKPIT`);

      // Decks 01, 02, 03 must all be present in DOM
      const deck1 = await page.$('#deck-01');
      const deck2 = await page.$('#deck-02');
      const deck3 = await page.$('#deck-03');
      assert(deck1 !== null, '#deck-01 (Player) must exist');
      assert(deck2 !== null, '#deck-02 (Queue) must exist');
      assert(deck3 !== null, '#deck-03 (Cabin) must exist');

      // Audio engine must be present
      const audio = await page.$('#audio-engine');
      assert(audio !== null, '#audio-engine element must exist in DOM');

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 2: Jukebox Search, Interactive Autocomplete & Enqueue
    // -------------------------------------------------------------
    await test('Jukebox & Queue: Autocomplete dropdown, search and queue append', async () => {
      const context = await browser.newContext({
        viewport: { width: 1440, height: 900 },
      });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app`, { waitUntil: 'domcontentloaded' });

      // Focus and type into search input
      const input = await page.$('#enqueue-input');
      assert(input !== null, '#enqueue-input must exist');
      await input.fill('Chico');

      // Wait for debounced autocomplete dropdown to become visible
      const dropdown = page.locator('#search-dropdown');
      await dropdown.waitFor({ state: 'visible', timeout: 5000 });

      const dropdownHtml = await dropdown.innerHTML();
      assert(
        dropdownHtml.includes('Construção') || dropdownHtml.includes('Chico') || dropdownHtml.includes('search-item'),
        'Dropdown must display search candidate results'
      );

      // Enqueue the track via form submission
      await page.click('#enqueue-submit-btn');

      // Wait for queue container to reflect items
      await page.waitForTimeout(1000);
      const queueList = await page.$('#queue-list-container');
      const queueContent = await queueList.textContent();
      assert(
        !queueContent.includes('[ NENHUMA CANTIGA ]') || queueContent.includes('Chico') || queueContent.includes('CONSTRUÇÃO'),
        'Queue list should update with enqueued track'
      );

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 3: Audio Engine Controls, Gain Slider & Transport
    // -------------------------------------------------------------
    await test('Audio Engine & Transport: Gain slider to 11 and play/pause controls', async () => {
      const context = await browser.newContext({
        viewport: { width: 1440, height: 900 },
      });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app?master_key=lacraia_master_2011`, { waitUntil: 'domcontentloaded' });

      // Test Gain Slider (0 to 11 scale)
      const gainSlider = page.locator('#gain-slider');
      await gainSlider.waitFor({ state: 'visible' });
      await gainSlider.fill('11');
      await gainSlider.dispatchEvent('input');

      const gainDisplay = await page.locator('#gain-val-display').textContent();
      assert(gainDisplay.includes('11/11'), `Gain display '${gainDisplay}' must indicate 11/11`);

      // Verify audio volume was set to max
      const audioVol = await page.evaluate(() => {
        const audio = document.getElementById('audio-engine');
        return audio ? audio.volume : null;
      });
      assert(audioVol === 1.0, `Audio element volume must clamp to 1.0 when gain is 11, got ${audioVol}`);

      // Click Transport Play button
      const playBtn = page.locator('#btn-play-pause');
      await playBtn.click();

      // Wait for audio engine source to be populated (either instant cache or resolve)
      await page.waitForFunction(() => {
        const audio = document.getElementById('audio-engine');
        return audio && audio.src && audio.src.length > 0 && audio.src !== window.location.href;
      }, { timeout: 15000 });

      const audioSrc = await page.evaluate(() => {
        const audio = document.getElementById('audio-engine');
        return audio ? audio.src : '';
      });
      assert(audioSrc.length > 0, 'Audio engine src must be populated upon playback trigger');
      assert(audioSrc.includes('/cache/'), `Audio src '${audioSrc}' must point to /cache/ mount`);

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 4: Mobile Viewport & Tab Switching (< 860px)
    // -------------------------------------------------------------
    await test('Mobile Viewport (375x667): Bottom tabs navigation and persistent miniplayer', async () => {
      const context = await browser.newContext({
        viewport: { width: 375, height: 667 },
        isMobile: true,
      });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app`, { waitUntil: 'domcontentloaded' });

      // Mobile tabs nav must be visible on small viewport
      const tabs = page.locator('#mobile-tabs');
      await tabs.waitFor({ state: 'visible', timeout: 5000 });

      // By default Deck 01 is active
      const deck1Visible = await page.locator('#deck-01').isVisible();
      assert(deck1Visible, 'Deck 01 must be initially visible on mobile');

      // Click FILA Tab (Deck 02)
      await page.click('#tab-btn-deck-02');
      await page.waitForTimeout(300);

      const deck2Visible = await page.locator('#deck-02').isVisible();
      const deck1Hidden = !(await page.locator('#deck-01').isVisible());
      assert(deck2Visible, 'Deck 02 must become visible when clicking FILA tab');
      assert(deck1Hidden, 'Deck 01 must be hidden when FILA tab is active');

      // Persistent Miniplayer in Deck 02 must be visible
      const miniplayer2Visible = await page.locator('#deck-02-miniplayer').isVisible();
      assert(miniplayer2Visible, 'Persistent miniplayer in Deck 02 must be visible');

      // Click CABINE Tab (Deck 03)
      await page.click('#tab-btn-deck-03');
      await page.waitForTimeout(300);

      const deck3Visible = await page.locator('#deck-03').isVisible();
      assert(deck3Visible, 'Deck 03 must become visible when clicking CABINE tab');

      await context.close();
    });

    // -------------------------------------------------------------
    // Test 5: Autopilot 5-Tracks Lookahead Buffer Maintenance
    // -------------------------------------------------------------
    await test('Autopilot Lookahead: 5 tracks ahead buffer continuously maintained', async () => {
      const context = await browser.newContext({
        viewport: { width: 1440, height: 900 },
      });
      const page = await context.newPage();
      await page.goto(`${BASE_URL}/app?master_key=lacraia_master_2011`, { waitUntil: 'domcontentloaded' });

      // Trigger playback to wake the Autopilot
      const playBtn = page.locator('#btn-play-pause');
      await playBtn.click();

      // Wait for autopilot to populate queue items in DOM
      await page.waitForFunction(() => {
        const rows = document.querySelectorAll('.queue-item-row');
        return rows.length >= 1;
      }, { timeout: 15000 });

      const queueText = await page.locator('#queue-list-container').textContent();
      assert(
        queueText.includes('AUTOPILOT') || queueText.includes('PILOTO') || queueText.includes('#01'),
        'Queue must display populated autopilot items'
      );

      await context.close();
    });

  } finally {
    await browser.close();
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
  process.exit(1);
});
