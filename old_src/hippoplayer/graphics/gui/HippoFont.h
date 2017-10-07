
#ifndef HIPPOFONT_H
#define HIPPOFONT_H

#include "core/Types.h"

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

enum HippoFontType
{
	HIPPOFONT_TYPE_MONOSPACE,
	HIPPOFONT_TYPE_VARIABLE,
};

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct HippoMonoFontLayout 
{
	uint16_t x;
	uint16_t y;
} HippoMonoFontlayout;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct HippoBitmapFont
{
	const char* name;
	void* fontData;
	void* layOut;
	void* userData; // used my platform implementation
	enum HippoFontType type;
	uint32_t firstCharOffset;
} HippoBitmapFont;

#endif