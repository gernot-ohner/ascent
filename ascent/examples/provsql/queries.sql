-- Each SELECT corresponds to a relation in ../provsql_compare.rs.
-- DISTINCT coalesces logical tuples; labels belong to the provenance mapping.
\echo @copy
SELECT x, y, sr_why(provenance(), 'labels')
FROM (SELECT DISTINCT source AS x, target AS y FROM edge) q;

\echo @join
SELECT x, y, sr_why(provenance(), 'labels')
FROM (SELECT DISTINCT a.source AS x, b.target AS y
      FROM edge a JOIN edge b ON a.target = b.source) q;

\echo @projection
SELECT x, y, sr_why(provenance(), 'labels')
FROM (SELECT DISTINCT source AS x, 0 AS y FROM edge) q;

\echo @alternatives
SELECT x, y, sr_why(provenance(), 'labels')
FROM (SELECT source AS x, target AS y FROM edge WHERE target = 1
      UNION
      SELECT source AS x, target AS y FROM edge WHERE source = 0) q;

\echo @self_join
SELECT x, y, sr_why(provenance(), 'labels')
FROM (SELECT DISTINCT a.source AS x, a.target AS y
      FROM edge a JOIN edge b ON a.source = b.source AND a.target = b.target) q;

\echo @non_absorption
SELECT x, y, sr_why(provenance(), 'labels')
FROM (SELECT source AS x, target AS y FROM edge
      UNION
      SELECT a.source AS x, b.target AS y
      FROM edge a JOIN edge b ON a.target = b.source) q;

\echo @alternative_product
WITH two_hop AS (
  SELECT DISTINCT a.source AS x, b.target AS y
  FROM edge a JOIN edge b ON a.target = b.source
)
SELECT x, y, sr_why(provenance(), 'labels')
FROM (SELECT DISTINCT a.x, a.y
      FROM two_hop a JOIN two_hop b ON a.x = b.x AND a.y = b.y) q;

\echo @no_match
SELECT x, y, sr_why(provenance(), 'labels')
FROM (SELECT DISTINCT source AS x, target AS y FROM edge WHERE source = 99) q;
