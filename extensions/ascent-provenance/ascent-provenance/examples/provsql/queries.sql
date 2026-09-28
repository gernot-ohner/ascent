-- Query bodies are materialized once, before changing any leaf valuations.
-- @copy
SELECT DISTINCT source AS x, target AS y FROM edge
-- @join
SELECT DISTINCT a.source AS x, b.target AS y
FROM edge a JOIN edge b ON a.target = b.source
-- @projection
SELECT DISTINCT source AS x, 0 AS y FROM edge
-- @alternatives
SELECT source AS x, target AS y FROM edge WHERE target = 1
UNION SELECT source AS x, target AS y FROM edge WHERE source = 0
-- @self_join
SELECT DISTINCT a.source AS x, a.target AS y
FROM edge a JOIN edge b ON a.source = b.source AND a.target = b.target
-- @non_absorption
SELECT source AS x, target AS y FROM edge
UNION SELECT a.source AS x, b.target AS y FROM edge a JOIN edge b ON a.target = b.source
-- @alternative_product
WITH two_hop AS (
  SELECT DISTINCT a.source AS x, b.target AS y FROM edge a JOIN edge b ON a.target = b.source
)
SELECT DISTINCT a.x, a.y FROM two_hop a JOIN two_hop b ON a.x = b.x AND a.y = b.y
-- @no_match
SELECT DISTINCT source AS x, target AS y FROM edge WHERE source = 99
-- @identity
-- Ordinary background has the multiplicative identity and no input label.
SELECT node AS x, node AS y, gate_one() AS provsql FROM destination
-- @reach
WITH RECURSIVE path(x,y) AS (
  SELECT source,target FROM edge WHERE EXISTS (SELECT 1 FROM recursive_enabled)
  UNION SELECT p.x,e.target FROM path p JOIN edge e ON p.y=e.source
) SELECT x,y FROM path
-- @suffix
WITH RECURSIVE path(x,y) AS (
  SELECT node,node FROM destination WHERE EXISTS (SELECT 1 FROM recursive_enabled)
  UNION SELECT e.source,p.y FROM edge e JOIN path p ON e.target=p.x
) SELECT x,y FROM path
