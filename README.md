# realtor_query

This is not meant to abuse the site in question. It is simply meant to avoid a lot of tedious manual searching of houses already identified the old-fashioned way of searching in the browser. All it does is identify the current status of the houses of interest. If the house ia active or pending, it also makes note of the price.

It reads from an input file in json format. The data looks like this:

{"name":"1234-A-St_City_ST_00000_M00000-00000","status":"active","price":"$000,000","query":"M0000000000"}

"query" is currently unused. The output file for one day can be renamed and become the input file for the next day.