import {describe, expect, it} from 'vitest';
import {
	formatIban,
	formatMoney,
	getWebsite,
	getWebsiteStats,
	isValidEmail,
	metaTag,
	monthLabel,
	normaliseDomain,
	normaliseIban,
	sumMoney,
	validateBic,
	validateDomain,
	validateIban,
} from './api';
import {demoWebsites} from './test/fixtures';

describe('validateIban', () => {
	it('accepts a well-formed German IBAN', () => {
		expect(validateIban('DE89370400440532013000')).toBeNull();
	});

	it('accepts the same IBAN written with spaces', () => {
		expect(validateIban('DE89 3704 0044 0532 0130 00')).toBeNull();
	});

	it('accepts a French IBAN containing a letter in the account number', () => {
		expect(validateIban('FR1420041010050500013M02606')).toBeNull();
	});

	it('rejects an IBAN whose check digits do not match', () => {
		expect(validateIban('DE89370400440532013001')).toMatch(/check digits/);
	});

	it('rejects a non-EU IBAN even when its checksum is valid', () => {
		expect(validateIban('GB82WEST12345698765432')).toMatch(/not an EU country/);
	});

	it('rejects an IBAN of the wrong length for its country', () => {
		expect(validateIban('DE8937040044053201300')).toMatch(/22 characters long/);
	});

	it('asks for an IBAN when the field is empty', () => {
		expect(validateIban('   ')).toMatch(/Enter the IBAN/);
	});

	it('rejects something that is not an IBAN at all', () => {
		expect(validateIban('hello there')).toMatch(/two letters and two digits/);
	});
});

describe('validateBic', () => {
	it('accepts an 11-character BIC', () => {
		expect(validateBic('COBADEFFXXX')).toBeNull();
	});

	it('accepts an 8-character BIC', () => {
		expect(validateBic('COBADEFF')).toBeNull();
	});

	it('rejects a BIC of the wrong shape', () => {
		expect(validateBic('COBA1')).toMatch(/8 or 11 characters/);
	});

	it('asks for a BIC when the field is empty', () => {
		expect(validateBic('')).toMatch(/Enter the BIC/);
	});
});

describe('IBAN formatting', () => {
	it('strips spaces and upper-cases for storage', () => {
		expect(normaliseIban('de89 3704 0044 0532 0130 00')).toBe(
			'DE89370400440532013000',
		);
	});

	it('groups in fours for display', () => {
		expect(formatIban('DE89370400440532013000')).toBe(
			'DE89 3704 0044 0532 0130 00',
		);
	});
});

describe('normaliseDomain', () => {
	it('strips scheme, www, path and query', () => {
		expect(normaliseDomain('https://www.Example.org/news/1?x=2')).toBe(
			'example.org',
		);
	});

	it('leaves a bare domain alone', () => {
		expect(normaliseDomain('  example.org  ')).toBe('example.org');
	});

	it('keeps a subdomain that is not www', () => {
		expect(normaliseDomain('http://blog.example.org')).toBe('blog.example.org');
	});
});

describe('validateDomain', () => {
	it('accepts a domain with a dot', () => {
		expect(validateDomain('example.org')).toBeNull();
	});

	it('rejects a single label', () => {
		expect(validateDomain('example')).toMatch(/does not look like a domain/);
	});

	it('asks for a domain when the field is empty', () => {
		expect(validateDomain('')).toMatch(/Enter the address/);
	});
});

describe('site verification', () => {
	it('builds a meta tag carrying the token', () => {
		const tag = metaTag('crv-1234abcd');

		expect(tag).toContain('creators-reward-site-verification');
		expect(tag).toContain('crv-1234abcd');
	});
});

describe('lookups', () => {
	it('finds a website by id', () => {
		expect(getWebsite(demoWebsites, '1')?.domain).toBe('daily-ledger.example');
	});

	it('returns nothing for an unknown website id', () => {
		expect(getWebsite(demoWebsites, '99')).toBeUndefined();
	});

	it('returns empty statistics for a site with no reads', () => {
		const stats = getWebsiteStats('99');

		expect(stats.views).toBe(0);
		expect(stats.months).toHaveLength(0);
	});
});

describe('formatting', () => {
	it('renders money as amount and currency', () => {
		expect(formatMoney({amount: '12.30', currency: 'EUR'})).toBe('12.30 EUR');
	});

	it('sums amounts in cents rather than floats', () => {
		expect(
			sumMoney([
				{amount: '0.10', currency: 'EUR'},
				{amount: '0.20', currency: 'EUR'},
			]),
		).toBe('0.30 EUR');
	});

	it('sums an empty list to zero', () => {
		expect(sumMoney([])).toBe('0.00 EUR');
	});

	it('labels a month key', () => {
		expect(monthLabel('2024-02')).toBe('Feb 2024');
	});

	it('accepts a plausible email', () => {
		expect(isValidEmail('owner@example.org')).toBe(true);
	});

	it('rejects an address with no domain', () => {
		expect(isValidEmail('owner@')).toBe(false);
	});
});
