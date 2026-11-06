import L from "leaflet";
import type { Bounds, MapInfo } from "./types";

/*
 * Coordinate handling copied from tarkov.dev's map page (MIT): a game position (x, z) is the
 * Leaflet point [lat = z, lng = x]; each map has a rotation and an affine "transform" that
 * set the pixel scale. Overlays are placed by their game-coordinate bounds.
 */

function applyRotation(latLng: L.LatLng, rotation: number): L.LatLng {
  if (!latLng.lng && !latLng.lat) return L.latLng(0, 0);
  const a = (rotation * Math.PI) / 180;
  const cos = Math.cos(a);
  const sin = Math.sin(a);
  const x = latLng.lng;
  const y = latLng.lat;
  return L.latLng(x * sin + y * cos, x * cos - y * sin);
}

export function makeCrs(info: Pick<MapInfo, "transform" | "rotation">): L.CRS {
  let [sx, mx, sy, my] = [1, 0, 1, 0];
  if (info.transform) {
    sx = info.transform[0];
    sy = info.transform[2] * -1;
    mx = info.transform[1];
    my = info.transform[3];
  }
  return L.Util.extend({}, L.CRS.Simple, {
    transformation: new L.Transformation(sx, mx, sy, my),
    projection: L.Util.extend({}, L.Projection.LonLat, {
      project: (latlng: L.LatLng) => L.Projection.LonLat.project(applyRotation(latlng, info.rotation)),
      unproject: (point: L.Point) => applyRotation(L.Projection.LonLat.unproject(point), info.rotation * -1),
    }),
  }) as L.CRS;
}

/** game bounds [[x, z], [x, z]] -> Leaflet bounds (lat = z, lng = x) */
export function toLatLngBounds(b: Bounds): L.LatLngBounds {
  return L.latLngBounds([b[0][1], b[0][0]], [b[1][1], b[1][0]]);
}

export const toLatLng = (x: number, z: number) => L.latLng(z, x);
