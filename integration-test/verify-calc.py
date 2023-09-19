import json


def check_file(expected, filename):
    with open(filename) as f:
        data = json.load(f)
    by_years = {}
    for r in data['curves'][0]['results']:
        years = tuple(r['period'])
        by_years[years] = r['vs_time']
    for exp in expected:
        year = int(exp['collectioncode'])
        years = (year, year + 20)
        got = by_years[years]
        assert got['above'] + got['below'] <= got['iter'] == 100000
        for side in ['above', 'below']:
            x1 = got[side]
            n1 = got['iter']
            p1 = (n1 - x1) / n1
            x2 = exp[side]
            n2 = exp['total']
            p2 = x2 / n2
            diff = abs(p1 - p2)
            # print(f'{filename} {years[0]}-{years[1]}: {side} {p1:.5f} vs. {p2:.5f} = {diff:.5f}')
            assert diff < 0.01


def main():
    # Expected values calculated with types2

    # select collectioncode, below, above, total
    # from result_p
    # where corpuscode = 'ceec-1680-1800'
    # and collectioncode in ('1680', '1700', '1720', '1740', '1760', '1780')
    # and datasetcode = 'ity'
    # and statcode = 'type-token'

    expected = [{
        "collectioncode": "1680",
        "below": 2799,
        "above": 9999968,
        "total": 10000000
    }, {
        "collectioncode": "1700",
        "below": 55770,
        "above": 9998164,
        "total": 10000000
    }, {
        "collectioncode": "1720",
        "below": 2308998,
        "above": 9526872,
        "total": 10000000
    }, {
        "collectioncode": "1740",
        "below": 5123116,
        "above": 6881167,
        "total": 10000000
    }, {
        "collectioncode": "1760",
        "below": 7785591,
        "above": 3967760,
        "total": 10000000
    }, {
        "collectioncode": "1780",
        "below": 9975743,
        "above": 95508,
        "total": 10000000
    }]
    check_file(expected, 'calc2/ceec-types-vs-tokens-ity.json')

    # ...
    # and datasetcode = 'ity'
    # and statcode = 'type-token'

    expected = [{
        "collectioncode": "1680",
        "below": 2984,
        "above": 9999634,
        "total": 10000000
    }, {
        "collectioncode": "1700",
        "below": 5661,
        "above": 9999177,
        "total": 10000000
    }, {
        "collectioncode": "1720",
        "below": 574869,
        "above": 9825665,
        "total": 10000000
    }, {
        "collectioncode": "1740",
        "below": 6041287,
        "above": 5381966,
        "total": 10000000
    }, {
        "collectioncode": "1760",
        "below": 8983111,
        "above": 1830900,
        "total": 10000000
    }, {
        "collectioncode": "1780",
        "below": 9982458,
        "above": 49657,
        "total": 10000000
    }]
    check_file(expected, 'calc2/ceec-types-vs-words-ity.json')

    # ...
    # and datasetcode = 'ness'
    # and statcode = 'type-word'

    expected = [{
        "collectioncode": "1680",
        "below": 2883376,
        "above": 8431444,
        "total": 10000000
    }, {
        "collectioncode": "1700",
        "below": 677193,
        "above": 9748131,
        "total": 10000000
    }, {
        "collectioncode": "1720",
        "below": 842568,
        "above": 9718137,
        "total": 10000000
    }, {
        "collectioncode": "1740",
        "below": 3738255,
        "above": 7662517,
        "total": 10000000
    }, {
        "collectioncode": "1760",
        "below": 9899264,
        "above": 295823,
        "total": 10000000
    }, {
        "collectioncode": "1780",
        "below": 6095141,
        "above": 5175685,
        "total": 10000000
    }]
    check_file(expected, 'calc2/ceec-types-vs-words-ness.json')

    # ...
    # and datasetcode = 'ness'
    # and statcode = 'type-token'

    expected = [{
        "collectioncode": "1680",
        "below": 4095137,
        "above": 8012456,
        "total": 10000000
    }, {
        "collectioncode": "1700",
        "below": 890356,
        "above": 9788934,
        "total": 10000000
    }, {
        "collectioncode": "1720",
        "below": 5583444,
        "above": 7383184,
        "total": 10000000
    }, {
        "collectioncode": "1740",
        "below": 6162314,
        "above": 6029467,
        "total": 10000000
    }, {
        "collectioncode": "1760",
        "below": 9861990,
        "above": 511105,
        "total": 10000000
    }, {
        "collectioncode": "1780",
        "below": 1777289,
        "above": 9082115,
        "total": 10000000
    }]
    check_file(expected, 'calc2/ceec-types-vs-tokens-ness.json')


main()
