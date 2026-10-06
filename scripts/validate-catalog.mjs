import { readFileSync } from 'node:fs';
import Ajv2020 from 'ajv/dist/2020.js';

const root = new URL('../lego-letterpress-kit/lego-letterpress-kit/', import.meta.url);
const catalog = JSON.parse(readFileSync(new URL('letterpress-pieces.json', root), 'utf8'));
const schema = JSON.parse(readFileSync(new URL('letterpress-pieces.schema.json', root), 'utf8'));
const validate = new Ajv2020({ allErrors: true }).compile(schema);
if (!validate(catalog)) {
  console.error(validate.errors);
  process.exitCode = 1;
} else {
  console.log(`Catalog schema valid: ${catalog.pieces.length} pieces.`);
}
