// Generated from letterpress-pieces.json
// Unofficial LEGO-letterpress print-surface library.

export type PieceCategory = 'rectilinear' | 'round' | 'curve' | 'wedge' | 'special';
export type GeometryFidelity = 'high' | 'close' | 'approximate';

export interface LetterpressPiece {
  id: string;
  designId: string;
  name: string;
  category: PieceCategory;
  footprint: { widthStuds: number; heightStuds: number };
  surfaceHeightPlates: number;
  geometry: {
    viewBox: [number, number, number, number];
    path: string;
    fillRule: 'nonzero' | 'evenodd';
    fidelity: GeometryFidelity;
  };
  gridMask: number[][];
  allowedRotations: number[];
  mirrorPartner: string | null;
  physicalLetterpressReady: boolean;
  letterpressImportance: number;
  defaultToolbar: boolean;
  thumbnail: string;
  notes: string;
  reference: string;
}

export const LETTERPRESS_META = {
  "schemaVersion": "1.0.0",
  "name": "LEGO Letterpress Digital Piece Kit",
  "purpose": "2D modular tile silhouettes for simulating LEGO-letterpress-style compositions on a stud grid.",
  "coordinateSystem": {
    "unit": "stud",
    "studPitchMmNominal": 8.0,
    "regularPieceGapMmNominal": 0.2,
    "regular1x1FootprintMmNominal": 7.8,
    "defaultSurfaceScale": 0.975,
    "rotationOrigin": "piece bounding-box center"
  },
  "rendering": {
    "recommendedGridStepStuds": 1,
    "recommendedDefaultBoard": {
      "widthStuds": 16,
      "heightStuds": 16
    },
    "collisionMode": "gridMask",
    "doNotAutoMergeAdjacentPieces": true,
    "recommendedPrintSimulation": {
      "inkCoverage": [
        0.84,
        0.99
      ],
      "edgeErosionStuds": [
        0.002,
        0.025
      ],
      "positionJitterStuds": [
        0.0,
        0.018
      ],
      "rotationJitterDegrees": [
        0.0,
        0.35
      ],
      "perPieceTexture": true,
      "globalPaperTexture": true,
      "registrationErrorForMultiColorStuds": [
        0.0,
        0.04
      ]
    }
  },
  "warnings": [
    "This is a print-surface simulation library, not manufacturing CAD.",
    "Rare/special pieces marked fidelity='approximate' should be replaced with LDraw-derived top projections if exact physical matching is required.",
    "Parts 68869 and 74169 are 2/3 brick high and should be excluded from a strict coplanar physical letterpress build unless height is compensated."
  ]
} as const;

export const LETTERPRESS_PIECES: LetterpressPiece[] = [
  {
    "id": "3070",
    "designId": "3070",
    "name": "Tile 1\u00d71",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 1,
      "heightStuds": 1
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        1,
        1
      ],
      "path": "M0 0 H1 V1 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 5,
    "defaultToolbar": true,
    "thumbnail": "thumbnails/3070.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/3070"
  },
  {
    "id": "3069",
    "designId": "3069",
    "name": "Tile 1\u00d72",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 1
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        1
      ],
      "path": "M0 0 H2 V1 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 5,
    "defaultToolbar": true,
    "thumbnail": "thumbnails/3069.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/3069"
  },
  {
    "id": "63864",
    "designId": "63864",
    "name": "Tile 1\u00d73",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 3,
      "heightStuds": 1
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        3,
        1
      ],
      "path": "M0 0 H3 V1 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 3,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/63864.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/63864"
  },
  {
    "id": "2431",
    "designId": "2431",
    "name": "Tile 1\u00d74",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 4,
      "heightStuds": 1
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        4,
        1
      ],
      "path": "M0 0 H4 V1 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 3,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/2431.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/2431"
  },
  {
    "id": "6636",
    "designId": "6636",
    "name": "Tile 1\u00d76",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 6,
      "heightStuds": 1
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        6,
        1
      ],
      "path": "M0 0 H6 V1 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 2,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/6636.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/6636"
  },
  {
    "id": "4162",
    "designId": "4162",
    "name": "Tile 1\u00d78",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 8,
      "heightStuds": 1
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        8,
        1
      ],
      "path": "M0 0 H8 V1 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 2,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/4162.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/4162"
  },
  {
    "id": "14719",
    "designId": "14719",
    "name": "Tile 2\u00d72 Corner",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        2
      ],
      "path": "M0 0 H2 V1 H1 V2 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1
      ],
      [
        1,
        0
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 3,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/14719.svg",
    "notes": "L-shaped 3-cell corner tile.",
    "reference": "https://brickarchitect.com/parts/14719"
  },
  {
    "id": "3068",
    "designId": "3068",
    "name": "Tile 2\u00d72",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        2
      ],
      "path": "M0 0 H2 V2 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1
      ],
      [
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 3,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/3068.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/3068"
  },
  {
    "id": "26603",
    "designId": "26603",
    "name": "Tile 2\u00d73",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 3,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        3,
        2
      ],
      "path": "M0 0 H3 V2 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1
      ],
      [
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 2,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/26603.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/26603"
  },
  {
    "id": "87079",
    "designId": "87079",
    "name": "Tile 2\u00d74",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 4,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        4,
        2
      ],
      "path": "M0 0 H4 V2 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 2,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/87079.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/87079"
  },
  {
    "id": "69729",
    "designId": "69729",
    "name": "Tile 2\u00d76",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 6,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        6,
        2
      ],
      "path": "M0 0 H6 V2 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 1,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/69729.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/69729"
  },
  {
    "id": "1751",
    "designId": "1751",
    "name": "Tile 4\u00d74",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 4,
      "heightStuds": 4
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        4,
        4
      ],
      "path": "M0 0 H4 V4 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 1,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/1751.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/1751"
  },
  {
    "id": "8165",
    "designId": "8165",
    "name": "Tile 4\u00d78",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 8,
      "heightStuds": 4
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        8,
        4
      ],
      "path": "M0 0 H8 V4 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 1,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/8165.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/8165"
  },
  {
    "id": "10202",
    "designId": "10202",
    "name": "Tile 6\u00d76",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 6,
      "heightStuds": 6
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        6,
        6
      ],
      "path": "M0 0 H6 V6 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 1,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/10202.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/10202"
  },
  {
    "id": "90498",
    "designId": "90498",
    "name": "Tile 8\u00d716",
    "category": "rectilinear",
    "footprint": {
      "widthStuds": 16,
      "heightStuds": 8
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        16,
        8
      ],
      "path": "M0 0 H16 V8 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 1,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/90498.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/90498"
  },
  {
    "id": "98138",
    "designId": "98138",
    "name": "Tile Round 1\u00d71",
    "category": "round",
    "footprint": {
      "widthStuds": 1,
      "heightStuds": 1
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        1,
        1
      ],
      "path": "M0.0 0.5 A0.5 0.5 0 1 0 1.0 0.5 A0.5 0.5 0 1 0 0.0 0.5 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 5,
    "defaultToolbar": true,
    "thumbnail": "thumbnails/98138.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/98138"
  },
  {
    "id": "24246",
    "designId": "24246",
    "name": "Tile 1\u00d71 with Rounded End",
    "category": "round",
    "footprint": {
      "widthStuds": 1,
      "heightStuds": 1
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        1,
        1
      ],
      "path": "M0 0 H0.5 A0.5 0.5 0 0 1 0.5 1 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 5,
    "defaultToolbar": true,
    "thumbnail": "thumbnails/24246.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/24246"
  },
  {
    "id": "1126",
    "designId": "1126",
    "name": "Tile Round 1\u00d72 Oval",
    "category": "round",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 1
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        1
      ],
      "path": "M0.5 0 H1.5 A0.5 0.5 0 0 1 2 0.5 V0.5 A0.5 0.5 0 0 1 1.5 1 H0.5 A0.5 0.5 0 0 1 0 0.5 V0.5 A0.5 0.5 0 0 1 0.5 0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 5,
    "defaultToolbar": true,
    "thumbnail": "thumbnails/1126.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/1126"
  },
  {
    "id": "14769",
    "designId": "14769",
    "name": "Tile Round 2\u00d72",
    "category": "round",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        2
      ],
      "path": "M0 1 A1 1 0 1 0 2 1 A1 1 0 1 0 0 1 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1
      ],
      [
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 4,
    "defaultToolbar": true,
    "thumbnail": "thumbnails/14769.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/14769"
  },
  {
    "id": "5520",
    "designId": "5520",
    "name": "Tile 2\u00d72 with Rounded End",
    "category": "round",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        2
      ],
      "path": "M0 0 H1.0 A1.0 1.0 0 0 1 1.0 2 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1
      ],
      [
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 3,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/5520.svg",
    "notes": "D-shaped tile, one straight end and one \u00d82 rounded end.",
    "reference": "https://brickarchitect.com/parts/5520"
  },
  {
    "id": "7853",
    "designId": "7853",
    "name": "Tile 2\u00d73 with Rounded Ends",
    "category": "round",
    "footprint": {
      "widthStuds": 3,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        3,
        2
      ],
      "path": "M1.0 0 H2.0 A1.0 1.0 0 0 1 3 1.0 V1.0 A1.0 1.0 0 0 1 2.0 2 H1.0 A1.0 1.0 0 0 1 0 1.0 V1.0 A1.0 1.0 0 0 1 1.0 0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1
      ],
      [
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 2,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/7853.svg",
    "notes": "Stored horizontally as a 3\u00d72 footprint for rendering.",
    "reference": "https://brickarchitect.com/parts/7853"
  },
  {
    "id": "66857",
    "designId": "66857",
    "name": "Tile 2\u00d74 with Rounded Ends",
    "category": "round",
    "footprint": {
      "widthStuds": 4,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        4,
        2
      ],
      "path": "M1.0 0 H3.0 A1.0 1.0 0 0 1 4 1.0 V1.0 A1.0 1.0 0 0 1 3.0 2 H1.0 A1.0 1.0 0 0 1 0 1.0 V1.0 A1.0 1.0 0 0 1 1.0 0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 3,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/66857.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/66857"
  },
  {
    "id": "25269",
    "designId": "25269",
    "name": "Tile Round 1\u00d71 Quarter",
    "category": "curve",
    "footprint": {
      "widthStuds": 1,
      "heightStuds": 1
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        1,
        1
      ],
      "path": "M0 0 H1 A1 1 0 0 1 0 1 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 5,
    "defaultToolbar": true,
    "thumbnail": "thumbnails/25269.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/25269"
  },
  {
    "id": "1748",
    "designId": "1748",
    "name": "Tile Round 1\u00d72 Half Circle",
    "category": "curve",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 1
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        1
      ],
      "path": "M0 1 A1 1 0 0 1 2 1 L0 1 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 5,
    "defaultToolbar": true,
    "thumbnail": "thumbnails/1748.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/1748"
  },
  {
    "id": "3396",
    "designId": "3396",
    "name": "Tile 2\u00d72 with Two Quarter-Round Cutouts",
    "category": "curve",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        2
      ],
      "path": "M1 0 H2 V1 A1 1 0 0 0 1 2 H0 V1 A1 1 0 0 0 1 0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1
      ],
      [
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 3,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/3396.svg",
    "notes": "Opposing quarter-circle concave cutouts. Useful for S-curves and pinched forms.",
    "reference": "https://brickarchitect.com/parts/3396"
  },
  {
    "id": "67095",
    "designId": "67095",
    "name": "Tile Round 3\u00d73",
    "category": "round",
    "footprint": {
      "widthStuds": 3,
      "heightStuds": 3
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        3,
        3
      ],
      "path": "M0.0 1.5 A1.5 1.5 0 1 0 3.0 1.5 A1.5 1.5 0 1 0 0.0 1.5 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1
      ],
      [
        1,
        1,
        1
      ],
      [
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 2,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/67095.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/67095"
  },
  {
    "id": "7212",
    "designId": "7212",
    "name": "Tile Round 6\u00d76",
    "category": "round",
    "footprint": {
      "widthStuds": 6,
      "heightStuds": 6
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        6,
        6
      ],
      "path": "M0 3 A3 3 0 1 0 6 3 A3 3 0 1 0 0 3 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 1,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/7212.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/7212"
  },
  {
    "id": "65474",
    "designId": "65474",
    "name": "Tile 6\u00d78 with Rounded Ends",
    "category": "round",
    "footprint": {
      "widthStuds": 8,
      "heightStuds": 6
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        8,
        6
      ],
      "path": "M3.0 0 H5.0 A3.0 3.0 0 0 1 8 3.0 V3.0 A3.0 3.0 0 0 1 5.0 6 H3.0 A3.0 3.0 0 0 1 0 3.0 V3.0 A3.0 3.0 0 0 1 3.0 0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 1,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/65474.svg",
    "notes": "Stored horizontally as an 8\u00d76 footprint for rendering.",
    "reference": "https://brickarchitect.com/parts/65474"
  },
  {
    "id": "74169",
    "designId": "74169",
    "name": "3\u00d74\u00d7\u2154 Mickey Mouse Tile",
    "category": "special",
    "footprint": {
      "widthStuds": 3,
      "heightStuds": 4
    },
    "surfaceHeightPlates": 2,
    "geometry": {
      "viewBox": [
        0,
        0,
        3,
        4
      ],
      "path": "M0.5 2.25 A1.0 1.0 0 1 0 2.5 2.25 A1.0 1.0 0 1 0 0.5 2.25 Z M0.15000000000000002 0.8 A0.5 0.5 0 1 0 1.15 0.8 A0.5 0.5 0 1 0 0.15000000000000002 0.8 Z M1.85 0.8 A0.5 0.5 0 1 0 2.85 0.8 A0.5 0.5 0 1 0 1.85 0.8 Z",
      "fillRule": "nonzero",
      "fidelity": "approximate"
    },
    "gridMask": [
      [
        1,
        1,
        1
      ],
      [
        1,
        1,
        1
      ],
      [
        1,
        1,
        1
      ],
      [
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": false,
    "letterpressImportance": 1,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/74169.svg",
    "notes": "Top-print silhouette approximation. This element is 2/3 brick high, so it is not coplanar with standard one-plate tiles without compensation.",
    "reference": "https://brickarchitect.com/parts/74169"
  },
  {
    "id": "27925",
    "designId": "27925",
    "name": "Tile 2\u00d72 Quarter Ring",
    "category": "curve",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        2
      ],
      "path": "M1 0 H2 A2 2 0 0 1 0 2 V1 A1 1 0 0 0 1 0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1
      ],
      [
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 5,
    "defaultToolbar": true,
    "thumbnail": "thumbnails/27925.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/27925"
  },
  {
    "id": "79393",
    "designId": "79393",
    "name": "Tile 3\u00d73 Quarter Ring",
    "category": "curve",
    "footprint": {
      "widthStuds": 3,
      "heightStuds": 3
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        3,
        3
      ],
      "path": "M2 0 H3 A3 3 0 0 1 0 3 V2 A2 2 0 0 0 2 0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1
      ],
      [
        1,
        1,
        1
      ],
      [
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 3,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/79393.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/79393"
  },
  {
    "id": "27507",
    "designId": "27507",
    "name": "Tile 4\u00d74 Quarter Ring",
    "category": "curve",
    "footprint": {
      "widthStuds": 4,
      "heightStuds": 4
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        4,
        4
      ],
      "path": "M3 0 H4 A4 4 0 0 1 0 4 V3 A3 3 0 0 0 3 0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 3,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/27507.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/27507"
  },
  {
    "id": "68869",
    "designId": "68869",
    "name": "Tile 4\u00d74\u00d7\u2154 with Rounded Corners",
    "category": "special",
    "footprint": {
      "widthStuds": 4,
      "heightStuds": 4
    },
    "surfaceHeightPlates": 2,
    "geometry": {
      "viewBox": [
        0,
        0,
        4,
        4
      ],
      "path": "M1 0 H3 A1 1 0 0 1 4 1 V3 A1 1 0 0 1 3 4 H1 A1 1 0 0 1 0 3 V1 A1 1 0 0 1 1 0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": false,
    "letterpressImportance": 2,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/68869.svg",
    "notes": "2/3 brick high. Keep in the digital library, but filter out of a strict one-height physical letterpress mode.",
    "reference": "https://brickarchitect.com/parts/68869"
  },
  {
    "id": "22385",
    "designId": "22385",
    "name": "Tile 2\u00d73 Pentagonal",
    "category": "wedge",
    "footprint": {
      "widthStuds": 3,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        3,
        2
      ],
      "path": "M0 0 H2 L3 1 L2 2 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1
      ],
      [
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 3,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/22385.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/22385"
  },
  {
    "id": "35787",
    "designId": "35787",
    "name": "Tile 2\u00d72 Triangular",
    "category": "wedge",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        2
      ],
      "path": "M0 0 L2 2 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1
      ],
      [
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 4,
    "defaultToolbar": true,
    "thumbnail": "thumbnails/35787.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/35787"
  },
  {
    "id": "27263",
    "designId": "27263",
    "name": "Tile 2\u00d72 Corner with 45\u00b0 Cut",
    "category": "wedge",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        2
      ],
      "path": "M0 0 H2 V1 H1.5 L1 1.5 V2 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "close"
    },
    "gridMask": [
      [
        1,
        1
      ],
      [
        1,
        0
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 3,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/27263.svg",
    "notes": "Letterpress top-surface approximation of the faceted corner tile.",
    "reference": "https://brickarchitect.com/parts/27263"
  },
  {
    "id": "5091",
    "designId": "5091",
    "name": "Tile 1\u00d72 45\u00b0 Corner Left",
    "category": "wedge",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 1
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        1
      ],
      "path": "M0 0 H2 V1 H1 L0 .5 Z",
      "fillRule": "nonzero",
      "fidelity": "approximate"
    },
    "gridMask": [
      [
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": "5092",
    "physicalLetterpressReady": true,
    "letterpressImportance": 2,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/5091.svg",
    "notes": "Simplified top silhouette for digital composition. Paired with 5092.",
    "reference": "https://brickarchitect.com/parts/5091"
  },
  {
    "id": "5092",
    "designId": "5092",
    "name": "Tile 1\u00d72 45\u00b0 Corner Right",
    "category": "wedge",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 1
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        1
      ],
      "path": "M0 0 H2 L2 .5 L1 1 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "approximate"
    },
    "gridMask": [
      [
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": "5091",
    "physicalLetterpressReady": true,
    "letterpressImportance": 2,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/5092.svg",
    "notes": "Simplified top silhouette for digital composition. Paired with 5091.",
    "reference": "https://brickarchitect.com/parts/5092"
  },
  {
    "id": "7828",
    "designId": "7828",
    "name": "63\u00b0 2\u00d72 Wedge Tile Left",
    "category": "wedge",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        2
      ],
      "path": "M0 0 H1.1 L2 2 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "approximate"
    },
    "gridMask": [
      [
        1,
        1
      ],
      [
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": "7829",
    "physicalLetterpressReady": true,
    "letterpressImportance": 2,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/7828.svg",
    "notes": "Approximate quadrilateral footprint. Keep isolated behind an 'extended pieces' toggle until you replace it with an LDraw-derived projection.",
    "reference": "https://brickarchitect.com/parts/7828"
  },
  {
    "id": "7829",
    "designId": "7829",
    "name": "63\u00b0 2\u00d72 Wedge Tile Right",
    "category": "wedge",
    "footprint": {
      "widthStuds": 2,
      "heightStuds": 2
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        2,
        2
      ],
      "path": "M.9 0 H2 V2 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "approximate"
    },
    "gridMask": [
      [
        1,
        1
      ],
      [
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": "7828",
    "physicalLetterpressReady": true,
    "letterpressImportance": 2,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/7829.svg",
    "notes": "Approximate mirrored quadrilateral footprint. Replace with an LDraw-derived projection for physical accuracy.",
    "reference": "https://brickarchitect.com/parts/7829"
  },
  {
    "id": "7975",
    "designId": "7975",
    "name": "Tile 4\u00d74 Triangular",
    "category": "wedge",
    "footprint": {
      "widthStuds": 4,
      "heightStuds": 4
    },
    "surfaceHeightPlates": 1,
    "geometry": {
      "viewBox": [
        0,
        0,
        4,
        4
      ],
      "path": "M0 0 L4 4 H0 Z",
      "fillRule": "nonzero",
      "fidelity": "high"
    },
    "gridMask": [
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ],
      [
        1,
        1,
        1,
        1
      ]
    ],
    "allowedRotations": [
      0,
      90,
      180,
      270
    ],
    "mirrorPartner": null,
    "physicalLetterpressReady": true,
    "letterpressImportance": 1,
    "defaultToolbar": false,
    "thumbnail": "thumbnails/7975.svg",
    "notes": "",
    "reference": "https://brickarchitect.com/parts/7975"
  }
];

export const DEFAULT_TOOLBAR = LETTERPRESS_PIECES.filter(p => p.defaultToolbar);
export const PHYSICAL_ONE_HEIGHT_SET = LETTERPRESS_PIECES.filter(
  p => p.physicalLetterpressReady && p.surfaceHeightPlates === 1
);
export const EXTENDED_SET = LETTERPRESS_PIECES;

export function getPiece(id: string): LetterpressPiece | undefined {
  return LETTERPRESS_PIECES.find(p => p.id === id);
}
