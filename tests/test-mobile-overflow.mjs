#!/usr/bin/env node
import { chromium } from './e2e/node_modules/playwright/index.mjs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const INDEX_PATH = `file://${path.resolve(__dirname, '../site/app/index.html')}`;

const VIEWPORTS = [320, 360, 375, 412];
const DECKS = ['deck-01', 'deck-02', 'deck-03'];

console.log('=================================================================');
console.log('🧪 [TESTE] Auditoria de Vazamento Horizontal Móvel (#35)');
console.log('=================================================================');

let totalFailures = 0;

const browser = await chromium.launch({ headless: true });

for (const width of VIEWPORTS) {
  const context = await browser.newContext({ viewport: { width, height: 740 } });
  const page = await context.newPage();
  await page.goto(INDEX_PATH);

  for (const deck of DECKS) {
    process.stdout.write(`-> Verificando ${width}px | Aba: ${deck}... `);

    // Ativar deck correspondente
    await page.evaluate((d) => {
      document.querySelectorAll('.mobile-tab-btn').forEach((b) => {
        if (b.dataset.deck === d) b.click();
      });
    }, deck);

    // Checar elementos com overflow lateral
    const overflows = await page.evaluate(() => {
      const docWidth = document.documentElement.clientWidth;
      const results = [];
      document.querySelectorAll('*').forEach((el) => {
        // Ignorar elementos ocultos
        const style = window.getComputedStyle(el);
        if (style.display === 'none' || style.visibility === 'hidden') return;
        
        const rect = el.getBoundingClientRect();
        if (rect.width > 0 && rect.right > docWidth + 0.5) {
          results.push({
            tag: el.tagName,
            id: el.id,
            cls: typeof el.className === 'string' ? el.className.trim() : '',
            right: Math.round(rect.right * 10) / 10,
            width: Math.round(rect.width * 10) / 10,
            docWidth
          });
        }
      });
      return results;
    });

    if (overflows.length === 0) {
      console.log('✅ OK (0 vazamentos)');
    } else {
      console.log(`❌ FALHOU (${overflows.length} elemento(s) vazando)`);
      overflows.slice(0, 3).forEach((o) => {
        console.log(`     -> <${o.tag.toLowerCase()} id="${o.id}" class="${o.cls}"> right=${o.right}px (max=${o.docWidth}px)`);
      });
      totalFailures++;
    }
  }
  await context.close();
}

await browser.close();

console.log('=================================================================');
if (totalFailures === 0) {
  console.log('🎉 NENHUM VAZAMENTO HORIZONTAL DETECTADO EM SMARTPHONES!');
  process.exit(0);
} else {
  console.log(`💥 ${totalFailures} CENÁRIO(S) APRESENTARAM TRANSBORDAMENTO.`);
  process.exit(1);
}
