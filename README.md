# realtor_query

This is not meant to abuse the site in question. It is simply meant to avoid a lot of tedious manual searching of houses already identified the old-fashioned way of searching in the browser. All it does is identify the current statuses of the houses of interest.

It reads from an input file in json format. The data looks like this:

{"name":"1234-A-St_City_ST_00000_M00000-00000","status":"active"}

The output file for one day can be renamed and become the input file for the next day.